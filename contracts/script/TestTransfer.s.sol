// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console2} from "forge-std/Script.sol";
import {ERC2771Forwarder} from "@openzeppelin/contracts/metatx/ERC2771Forwarder.sol";
import {GaslessToken} from "../src/GaslessToken.sol";

/**
 * @notice End-to-end gasless transfer script using OZ ERC2771Forwarder.
 *
 * Usage:
 *   export FORWARDER_ADDRESS=0x...  TOKEN_ADDRESS=0x...
 *   export USER_PRIVATE_KEY=0x...   RELAYER_PRIVATE_KEY=0x...
 *   forge script script/TestTransfer.s.sol --rpc-url $RPC_URL --broadcast
 */
contract TestTransferScript is Script {
    bytes32 constant DOMAIN_TYPEHASH = keccak256(
        "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
    );
    // OZ v5.6.1: nonce is in the typehash, deadline is uint48
    bytes32 constant FORWARD_TYPEHASH = keccak256(
        "ForwardRequest(address from,address to,uint256 value,uint256 gas,uint256 nonce,uint48 deadline,bytes data)"
    );

    function run() public {
        address forwarderAddr = vm.envAddress("FORWARDER_ADDRESS");
        address tokenAddr    = vm.envAddress("TOKEN_ADDRESS");
        uint256 userPk       = vm.envUint("USER_PRIVATE_KEY");
        uint256 relayerPk    = vm.envUint("RELAYER_PRIVATE_KEY");
        address user         = vm.addr(userPk);
        address recipient    = vm.envOr("RECIPIENT_ADDRESS", address(0xdead));
        uint256 amount       = 100 ether;

        ERC2771Forwarder forwarder = ERC2771Forwarder(forwarderAddr);
        GaslessToken token = GaslessToken(tokenAddr);

        // Relayer mints tokens to user
        vm.startBroadcast(relayerPk);
        token.mint(user, amount * 10);
        vm.stopBroadcast();
        console2.log("Minted to user:", user);

        // Build request
        ERC2771Forwarder.ForwardRequestData memory req = ERC2771Forwarder.ForwardRequestData({
            from:      user,
            to:        tokenAddr,
            value:     0,
            gas:       200_000,
            deadline:  uint48(block.timestamp + 1 hours),
            data:      abi.encodeCall(token.transfer, (recipient, amount)),
            signature: new bytes(0)
        });

        // Compute domain separator (OZ v5 has no public DOMAIN_SEPARATOR())
        bytes32 domainSep = keccak256(abi.encode(
            DOMAIN_TYPEHASH,
            keccak256(bytes("GasRelayForwarder")),
            keccak256(bytes("1")),
            block.chainid,
            forwarderAddr
        ));

        // EIP-712 sign
        uint256 nonce = forwarder.nonces(user);
        bytes32 structHash = keccak256(abi.encode(
            FORWARD_TYPEHASH,
            req.from, req.to, req.value, req.gas,
            nonce, req.deadline, keccak256(req.data)
        ));
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", domainSep, structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(userPk, digest);
        req.signature = abi.encodePacked(r, s, v);

        uint256 before = token.balanceOf(recipient);
        console2.log("Recipient balance before:", before);

        // Relayer submits — user pays no gas
        vm.startBroadcast(relayerPk);
        forwarder.execute(req);
        vm.stopBroadcast();

        uint256 after_ = token.balanceOf(recipient);
        console2.log("Recipient balance after: ", after_);
        require(after_ == before + amount, "transfer amount mismatch");
        console2.log("SUCCESS: gasless transfer via OZ ERC2771Forwarder");
    }
}
