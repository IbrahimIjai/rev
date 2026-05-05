// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console2} from "forge-std/Test.sol";
import {ERC2771Forwarder} from "@openzeppelin/contracts/metatx/ERC2771Forwarder.sol";
import {GaslessToken} from "../src/GaslessToken.sol";

contract ForwarderTest is Test {
    ERC2771Forwarder public forwarder;
    GaslessToken public token;

    uint256 internal userPk = 0xA11CE;
    address internal user;
    address internal relayer = address(0x1234);

    bytes32 constant DOMAIN_TYPEHASH = keccak256(
        "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
    );

    /// OZ v5.6.1: nonce IS in the typehash, deadline is uint48
    bytes32 constant FORWARD_TYPEHASH = keccak256(
        "ForwardRequest(address from,address to,uint256 value,uint256 gas,uint256 nonce,uint48 deadline,bytes data)"
    );

    function setUp() public {
        forwarder = new ERC2771Forwarder("GasRelayForwarder");
        token = new GaslessToken("GaslessToken", "GLT", address(forwarder));
        user = vm.addr(userPk);
        token.mint(user, 1000 ether);
    }

    function _domainSeparator() internal view returns (bytes32) {
        return keccak256(abi.encode(
            DOMAIN_TYPEHASH,
            keccak256(bytes("GasRelayForwarder")),
            keccak256(bytes("1")),
            block.chainid,
            address(forwarder)
        ));
    }

    function _signRequest(
        ERC2771Forwarder.ForwardRequestData memory req
    ) internal view returns (bytes memory sig) {
        uint256 nonce = forwarder.nonces(req.from);
        bytes32 structHash = keccak256(abi.encode(
            FORWARD_TYPEHASH,
            req.from,
            req.to,
            req.value,
            req.gas,
            nonce,          // nonce in hash but not in struct
            req.deadline,
            keccak256(req.data)
        ));
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", _domainSeparator(), structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(userPk, digest);
        return abi.encodePacked(r, s, v);
    }

    function test_direct_transfer() public {
        vm.prank(user);
        bool ok = token.transfer(address(0xBEEF), 100 ether);
        assertTrue(ok);
        assertEq(token.balanceOf(address(0xBEEF)), 100 ether);
    }

    function test_gasless_transfer_via_forwarder() public {
        address recipient = address(0xBEEF);
        uint256 amount = 100 ether;

        ERC2771Forwarder.ForwardRequestData memory req = ERC2771Forwarder.ForwardRequestData({
            from:      user,
            to:        address(token),
            value:     0,
            gas:       100_000,
            deadline:  uint48(block.timestamp + 1 hours),
            data:      abi.encodeCall(token.transfer, (recipient, amount)),
            signature: new bytes(0)
        });
        req.signature = _signRequest(req);

        vm.prank(relayer);
        forwarder.execute(req);

        assertEq(token.balanceOf(recipient), amount);
        assertEq(token.balanceOf(user), 1000 ether - amount);
    }

    function test_nonce_increments_after_execute() public {
        uint256 before = forwarder.nonces(user);
        test_gasless_transfer_via_forwarder();
        assertEq(forwarder.nonces(user), before + 1);
    }

    function test_expired_request_reverts() public {
        ERC2771Forwarder.ForwardRequestData memory req = ERC2771Forwarder.ForwardRequestData({
            from:      user,
            to:        address(token),
            value:     0,
            gas:       100_000,
            deadline:  uint48(block.timestamp - 1),
            data:      abi.encodeCall(token.transfer, (address(0xBEEF), 1 ether)),
            signature: new bytes(65)
        });

        vm.prank(relayer);
        vm.expectRevert();
        forwarder.execute(req);
    }

    function test_wrong_nonce_reverts() public {
        ERC2771Forwarder.ForwardRequestData memory req = ERC2771Forwarder.ForwardRequestData({
            from:      user,
            to:        address(token),
            value:     0,
            gas:       100_000,
            deadline:  uint48(block.timestamp + 1 hours),
            data:      abi.encodeCall(token.transfer, (address(0xBEEF), 1 ether)),
            signature: new bytes(0)
        });

        // Sign with wrong nonce (current + 1)
        uint256 wrongNonce = forwarder.nonces(user) + 1;
        bytes32 structHash = keccak256(abi.encode(
            FORWARD_TYPEHASH,
            req.from, req.to, req.value, req.gas,
            wrongNonce, req.deadline, keccak256(req.data)
        ));
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", _domainSeparator(), structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(userPk, digest);
        req.signature = abi.encodePacked(r, s, v);

        vm.prank(relayer);
        vm.expectRevert();
        forwarder.execute(req);
    }
}
