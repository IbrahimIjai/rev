// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {Script, console2} from "forge-std/Script.sol";
import {MinimalForwarder} from "../src/MinimalForwarder.sol";
import {ExampleGaslessToken} from "../src/ExampleGaslessToken.sol";

contract DeployScript is Script {
    function run() public {
        vm.startBroadcast();

        MinimalForwarder forwarder = new MinimalForwarder();
        console2.log("MinimalForwarder deployed at:", address(forwarder));

        ExampleGaslessToken token = new ExampleGaslessToken(
            "GaslessToken",
            "GLT",
            address(forwarder)
        );
        console2.log("ExampleGaslessToken deployed at:", address(token));

        vm.stopBroadcast();
    }
}
