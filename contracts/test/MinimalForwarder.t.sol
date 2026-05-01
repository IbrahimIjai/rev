// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {Test, console2} from "forge-std/Test.sol";
import {MinimalForwarder} from "../src/MinimalForwarder.sol";

contract MinimalForwarderTest is Test {
    MinimalForwarder public forwarder;
    uint256 public userPrivateKey;
    address public user;

    function setUp() public {
        forwarder = new MinimalForwarder();
        userPrivateKey = 0x1234;
        user = vm.addr(userPrivateKey);
    }

    function test_Execute() public {
        MinimalForwarder.ForwardRequest memory req = MinimalForwarder.ForwardRequest({
            from: user,
            to: address(this),
            value: 0,
            gas: 100000,
            nonce: 0,
            data: abi.encodeWithSignature("targetFunction()")
        });

        bytes32 domainSeparator = forwarder.domainSeparator();
        bytes32 structHash = keccak256(
            abi.encode(
                keccak256("ForwardRequest(address from,address to,uint256 value,uint256 gas,uint256 nonce,bytes data)"),
                req.from,
                req.to,
                req.value,
                req.gas,
                req.nonce,
                keccak256(req.data)
            )
        );
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", domainSeparator, structHash));

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(userPrivateKey, digest);
        bytes memory signature = abi.encodePacked(r, s, v);

        (bool success, ) = forwarder.execute(req, signature);
        assertTrue(success, "Execution failed");
        assertEq(forwarder.getNonce(user), 1, "Nonce not incremented");
    }

    function test_ExecuteWithDeadline() public {
        MinimalForwarder.ForwardRequestWithDeadline memory req = MinimalForwarder.ForwardRequestWithDeadline({
            from: user,
            to: address(this),
            value: 0,
            gas: 100000,
            nonce: 0,
            deadline: block.timestamp + 100,
            data: abi.encodeWithSignature("targetFunction()")
        });

        bytes32 domainSeparator = forwarder.domainSeparator();
        bytes32 structHash = keccak256(
            abi.encode(
                keccak256("ForwardRequestWithDeadline(address from,address to,uint256 value,uint256 gas,uint256 nonce,uint256 deadline,bytes data)"),
                req.from,
                req.to,
                req.value,
                req.gas,
                req.nonce,
                req.deadline,
                keccak256(req.data)
            )
        );
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", domainSeparator, structHash));

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(userPrivateKey, digest);
        bytes memory signature = abi.encodePacked(r, s, v);

        (bool success, ) = forwarder.execute(req, signature);
        assertTrue(success, "Execution failed");
        assertEq(forwarder.getNonce(user), 1, "Nonce not incremented");
    }

    function test_Revert_ExpiredDeadline() public {
        MinimalForwarder.ForwardRequestWithDeadline memory req = MinimalForwarder.ForwardRequestWithDeadline({
            from: user,
            to: address(this),
            value: 0,
            gas: 100000,
            nonce: 0,
            deadline: block.timestamp - 1,
            data: abi.encodeWithSignature("targetFunction()")
        });

        bytes32 domainSeparator = forwarder.domainSeparator();
        bytes32 structHash = keccak256(
            abi.encode(
                keccak256("ForwardRequestWithDeadline(address from,address to,uint256 value,uint256 gas,uint256 nonce,uint256 deadline,bytes data)"),
                req.from,
                req.to,
                req.value,
                req.gas,
                req.nonce,
                req.deadline,
                keccak256(req.data)
            )
        );
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", domainSeparator, structHash));

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(userPrivateKey, digest);
        bytes memory signature = abi.encodePacked(r, s, v);

        vm.expectRevert("MinimalForwarder: signature mismatch or expired");
        forwarder.execute(req, signature);
    }

    function targetFunction() public pure returns (bool) {
        return true;
    }
}
