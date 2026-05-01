// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {ECDSA} from "./libraries/ECDSA.sol";

contract MinimalForwarder {
    using ECDSA for bytes32;


    bytes32 private constant _TYPEHASH =
        keccak256(
            "ForwardRequest(address from,address to,uint256 value,uint256 gas,uint256 nonce,bytes data)"
        );

    bytes32 private constant _TYPEHASH_WITH_DEADLINE =
        keccak256(
            "ForwardRequestWithDeadline(address from,address to,uint256 value,uint256 gas,uint256 nonce,uint256 deadline,bytes data)"
        );

    bytes32 private immutable _DOMAIN_SEPARATOR;

    constructor() {
        _DOMAIN_SEPARATOR = keccak256(
            abi.encode(
                keccak256(
                    "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
                ),
                keccak256("MinimalForwarder"),
                keccak256("0.0.1"),
                block.chainid,
                address(this)
            )
        );
    }


    struct ForwardRequest {
        address from;
        address to;
        uint256 value;
        uint256 gas;
        uint256 nonce;
        bytes data;
    }

    struct ForwardRequestWithDeadline {
        address from;
        address to;
        uint256 value;
        uint256 gas;
        uint256 nonce;
        uint256 deadline;
        bytes data;
    }


    mapping(address => uint256) private _nonces;

    function getNonce(address from) public view returns (uint256) {
        return _nonces[from];
    }


    function verify(
        ForwardRequest calldata req,
        bytes calldata signature
    ) public view returns (bool) {
        address signer = _hashTypedDataV4(
            keccak256(
                abi.encode(
                    _TYPEHASH,
                    req.from,
                    req.to,
                    req.value,
                    req.gas,
                    req.nonce,
                    keccak256(req.data)
                )
            )
        ).recover(signature);

        return _nonces[req.from] == req.nonce && signer == req.from;
    }

    function verify(
        ForwardRequestWithDeadline calldata req,
        bytes calldata signature
    ) public view returns (bool) {
        if (block.timestamp > req.deadline) {
            return false;
        }

        address signer = _hashTypedDataV4(
            keccak256(
                abi.encode(
                    _TYPEHASH_WITH_DEADLINE,
                    req.from,
                    req.to,
                    req.value,
                    req.gas,
                    req.nonce,
                    req.deadline,
                    keccak256(req.data)
                )
            )
        ).recover(signature);

        return _nonces[req.from] == req.nonce && signer == req.from;
    }


    function execute(
        ForwardRequest calldata req,
        bytes calldata signature
    ) public payable returns (bool success, bytes memory returndata) {
        require(verify(req, signature), "MinimalForwarder: signature mismatch");

        _nonces[req.from]++;

        (success, returndata) = req.to.call{gas: req.gas, value: req.value}(
            abi.encodePacked(req.data, req.from)
        );

        assert(gasleft() > req.gas / 63);

        return (success, returndata);
    }

    function execute(
        ForwardRequestWithDeadline calldata req,
        bytes calldata signature
    ) public payable returns (bool success, bytes memory returndata) {
        require(verify(req, signature), "MinimalForwarder: signature mismatch or expired");

        _nonces[req.from]++;

        (success, returndata) = req.to.call{gas: req.gas, value: req.value}(
            abi.encodePacked(req.data, req.from)
        );

        assert(gasleft() > req.gas / 63);

        return (success, returndata);
    }


    function domainSeparator() public view returns (bytes32) {
        return _DOMAIN_SEPARATOR;
    }

    function _hashTypedDataV4(bytes32 structHash) internal view returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", _DOMAIN_SEPARATOR, structHash));
    }
}
