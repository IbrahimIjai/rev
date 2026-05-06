// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console2} from "forge-std/Script.sol";
import {ERC2771Forwarder} from "@openzeppelin/contracts/metatx/ERC2771Forwarder.sol";
import {GaslessToken} from "../src/GaslessToken.sol";
import {GaslessNFT} from "../src/GaslessNFT.sol";

/**
 * @notice Deploys:
 *   1. OZ ERC2771Forwarder  — the shared trusted forwarder for all projects
 *      on this chain. Pass this address to the relayer as FORWARDER_ADDRESS.
 *   2. GaslessToken          — example target contract that uses ERC2771Context.
 *      dApp developers deploy their own contracts; this is for testing only.
 *
 * Usage:
 *   forge script script/Deploy.s.sol --rpc-url $RPC_URL --broadcast \
 *     --verify --verifier blockscout \
 *     --verifier-url https://base-sepolia.blockscout.com/api/
 */
contract DeployScript is Script {
    function run() public {
        vm.startBroadcast();

        // The name becomes the EIP-712 domain name — must match DOMAIN_NAME env var
        ERC2771Forwarder forwarder = new ERC2771Forwarder("GasRelayForwarder");
        console2.log("ERC2771Forwarder:", address(forwarder));

        GaslessToken token = new GaslessToken(
            "GaslessToken",
            "GLT",
            address(forwarder)
        );
        console2.log("GaslessToken:     ", address(token));

        GaslessNFT nft = new GaslessNFT(
            "GaslessNFT",
            "GNFT",
            address(forwarder)
        );
        console2.log("GaslessNFT:       ", address(nft));

        vm.stopBroadcast();
    }
}
