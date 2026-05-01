// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {Script, console2} from "forge-std/Script.sol";
import {MinimalForwarder} from "../src/MinimalForwarder.sol";
import {ExampleGaslessToken} from "../src/ExampleGaslessToken.sol";

contract TestTransferScript is Script {
    // ARC testnet chain ID
    uint256 constant CHAIN_ID = 5042002;

    function run() public {
        address forwarderAddr = vm.envAddress("FORWARDER_ADDRESS");
        address tokenAddr = vm.envAddress("TOKEN_ADDRESS");

        // User wallet — signs the meta-tx but holds no gas
        uint256 userPrivateKey = vm.envUint("USER_PRIVATE_KEY");
        address user = vm.addr(userPrivateKey);

        // Recipient for the gasless transfer
        address recipient = vm.envOr("RECIPIENT_ADDRESS", address(0xdead));

        uint256 transferAmount = 100 ether;

        MinimalForwarder forwarder = MinimalForwarder(forwarderAddr);
        ExampleGaslessToken token = ExampleGaslessToken(tokenAddr);

        // Relayer mints tokens to the user first
        uint256 relayerKey = vm.envUint("RELAYER_PRIVATE_KEY");
        vm.startBroadcast(relayerKey);
        token.mint(user, transferAmount * 10);
        console2.log("Minted", transferAmount * 10, "tokens to user:", user);
        vm.stopBroadcast();

        // Build the EIP-712 ForwardRequest for a gasless transfer
        uint256 nonce = forwarder.getNonce(user);
        uint256 deadline = block.timestamp + 1 hours;

        // Encode transfer(recipient, amount)
        bytes memory transferData = abi.encodeWithSignature(
            "transfer(address,uint256)",
            recipient,
            transferAmount
        );

        MinimalForwarder.ForwardRequestWithDeadline memory req = MinimalForwarder.ForwardRequestWithDeadline({
            from: user,
            to: tokenAddr,
            value: 0,
            gas: 200_000,
            nonce: nonce,
            deadline: deadline,
            data: transferData
        });

        // Sign the request as the user (no ETH needed)
        bytes32 domainSep = forwarder.domainSeparator();
        bytes32 typeHash = keccak256(
            "ForwardRequestWithDeadline(address from,address to,uint256 value,uint256 gas,uint256 nonce,uint256 deadline,bytes data)"
        );
        bytes32 structHash = keccak256(
            abi.encode(
                typeHash,
                req.from,
                req.to,
                req.value,
                req.gas,
                req.nonce,
                req.deadline,
                keccak256(req.data)
            )
        );
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", domainSep, structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(userPrivateKey, digest);
        bytes memory signature = abi.encodePacked(r, s, v);

        // Balances before
        uint256 userBalBefore = token.balanceOf(user);
        uint256 recipientBalBefore = token.balanceOf(recipient);
        console2.log("User balance before:      ", userBalBefore);
        console2.log("Recipient balance before: ", recipientBalBefore);

        // Relayer submits the meta-tx on behalf of the user
        vm.startBroadcast(relayerKey);
        (bool success,) = forwarder.execute(req, signature);
        require(success, "TestTransfer: meta-tx reverted");
        vm.stopBroadcast();

        // Balances after
        uint256 userBalAfter = token.balanceOf(user);
        uint256 recipientBalAfter = token.balanceOf(recipient);
        console2.log("User balance after:       ", userBalAfter);
        console2.log("Recipient balance after:  ", recipientBalAfter);

        require(userBalAfter == userBalBefore - transferAmount, "user balance mismatch");
        require(recipientBalAfter == recipientBalBefore + transferAmount, "recipient balance mismatch");

        console2.log("Gasless transfer SUCCESS");
    }
}
