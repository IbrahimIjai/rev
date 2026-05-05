use crate::models::ForwardRequest;
use crate::services::{
    nonce_manager::NonceService,
    policy::PolicyEnforcer,
    relay_executor::RelayExecutor,
};
use anyhow::{Context, Result};
use chrono::Utc;
use alloy::primitives::{Bytes, B256, U256};
use alloy::network::Ethereum;
use alloy::transports::Transport;
use alloy::providers::Provider;
use std::sync::Arc;
use std::time::Duration;
use tokio::{sync::mpsc, time::sleep};
use tracing::{error, info, warn};
use uuid::Uuid;

const MAX_RETRIES: u32 = 3;
const BASE_RETRY_DELAY_SECS: u64 = 5;

#[derive(Debug, Clone)]
pub struct QueuedJob {
    pub id: Uuid,
    pub project_id: Uuid,
    pub chain_id: u64,
    pub request: ForwardRequest,
    pub signature: Bytes,
    pub attempts: u32,
    pub created_at: chrono::DateTime<Utc>,
    pub next_attempt_at: chrono::DateTime<Utc>,
}

#[derive(Debug)]
pub struct RelayQueue {
    sender: mpsc::Sender<QueuedJob>,
}

impl RelayQueue {
    pub fn new(capacity: usize) -> (Self, mpsc::Receiver<QueuedJob>) {
        let (sender, receiver) = mpsc::channel(capacity);
        (Self { sender }, receiver)
    }

    pub async fn enqueue(&self, job: QueuedJob) -> Result<()> {
        self.sender.send(job).await.context("queue is full")?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProcessorContext<P, T = alloy::transports::BoxTransport>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    pub executor: Arc<RelayExecutor<P, T>>,
    pub nonce_service: Arc<NonceService<P, T>>,
    pub policy_enforcer: Arc<PolicyEnforcer>,
}

pub async fn process_job<P, T>(ctx: &ProcessorContext<P, T>, job: &QueuedJob) -> ProcessResult
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    let req = &job.request;
    let sig = &job.signature;

    let relayer_nonce = match ctx
        .nonce_service
        .relayer_manager
        .acquire_nonce(&*ctx.nonce_service.provider)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            return ProcessResult::RetryableFailure(format!("failed to acquire nonce: {}", e))
        }
    };

    let tx_hash = match ctx.executor.submit(req, sig, relayer_nonce).await {
        Ok(h) => {
            ctx.nonce_service.relayer_manager.confirm_nonce(relayer_nonce).await;
            h
        }
        Err(e) => {
            warn!(error = %e, "transaction submission failed");
            let _ = ctx
                .nonce_service
                .relayer_manager
                .reset_from_chain(&*ctx.nonce_service.provider)
                .await;
            return ProcessResult::RetryableFailure(format!("submission failed: {}", e));
        }
    };

    match ctx.executor.wait_for_confirmation(tx_hash).await {
        Ok(receipt) => {
            ctx.nonce_service.invalidate_user_nonce(req.from);
            let actual_gas = U256::from(receipt.gas_used);
            ctx.policy_enforcer
                .record_actual_gas_used(req.from, req.gas, actual_gas)
                .await;
            ProcessResult::Success {
                tx_hash,
                block_number: receipt.block_number,
                gas_used: Some(actual_gas),
            }
        }
        Err(e) => {
            if e.to_string().contains("reverted") {
                ctx.policy_enforcer.refund_quota(req.from, req.gas).await;
                ProcessResult::PermanentFailure(format!("reverted: {}", e))
            } else {
                ProcessResult::RetryableFailure(format!("confirmation failed: {}", e))
            }
        }
    }
}

#[derive(Debug)]
pub enum ProcessResult {
    Success {
        tx_hash: B256,
        block_number: Option<u64>,
        gas_used: Option<U256>,
    },
    RetryableFailure(String),
    PermanentFailure(String),
}

pub async fn run_worker<P, T>(
    mut receiver: mpsc::Receiver<QueuedJob>,
    retry_sender: mpsc::Sender<QueuedJob>,
    ctx: Arc<ProcessorContext<P, T>>,
    worker_id: usize,
) where
    P: Provider<T, Ethereum> + 'static,
    T: Transport + Clone + 'static,
{
    while let Some(job) = receiver.recv().await {
        info!(worker = worker_id, job_id = %job.id, "processing job");
        match process_job(&ctx, &job).await {
            ProcessResult::Success { tx_hash, .. } => {
                info!(job_id = %job.id, tx_hash = ?tx_hash, "job confirmed");
            }
            ProcessResult::RetryableFailure(reason) => {
                let attempts = job.attempts + 1;
                if attempts < MAX_RETRIES {
                    let delay = BASE_RETRY_DELAY_SECS * (2u64.pow(attempts));
                    warn!(job_id = %job.id, attempt = attempts, delay, "retrying job");
                    let retry_job = QueuedJob {
                        attempts,
                        next_attempt_at: Utc::now()
                            + chrono::Duration::seconds(delay as i64),
                        ..job
                    };
                    let rs = retry_sender.clone();
                    tokio::spawn(async move {
                        sleep(Duration::from_secs(delay)).await;
                        let _ = rs.send(retry_job).await;
                    });
                } else {
                    error!(job_id = %job.id, "max retries reached: {}", reason);
                }
            }
            ProcessResult::PermanentFailure(reason) => {
                error!(job_id = %job.id, "job permanently failed: {}", reason);
            }
        }
    }
}

pub async fn spawn_worker_pool<P, T>(
    receiver: mpsc::Receiver<QueuedJob>,
    ctx: Arc<ProcessorContext<P, T>>,
    _num_workers: usize,
) where
    P: Provider<T, Ethereum> + 'static,
    T: Transport + Clone + 'static,
{
    let (retry_sender, retry_receiver) = mpsc::channel(1000);
    let ctx_a = ctx.clone();
    let rs_a = retry_sender.clone();
    tokio::spawn(async move { run_worker(receiver, rs_a, ctx_a, 0).await });
    tokio::spawn(async move { run_worker(retry_receiver, retry_sender, ctx, 1).await });
}
