// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/// @dev Elliptic Curve Digital Signature Algorithm (ECDSA) operations.
library ECDSA {
    enum RecoverError {
        NoError,
        InvalidSignature,
        InvalidSignatureLength,
        InvalidSignatureS
    }

    /**
     * @dev Overload of {ECDSA-recover} that receives the `v`, `r` and `s` signature fields separately.
     */
    function recover(bytes32 hash, uint8 v, bytes32 r, bytes32 s) internal pure returns (address) {
        (address recovered, RecoverError error) = tryRecover(hash, v, r, s);
        if (error != RecoverError.NoError) {
            revert("ECDSA: invalid signature");
        }
        return recovered;
    }

    /**
     * @dev Overload of {ECDSA-recover} that receives the `signature` as an array of bytes.
     */
    function recover(bytes32 hash, bytes memory signature) internal pure returns (address) {
        (address recovered, RecoverError error) = tryRecover(hash, signature);
        if (error != RecoverError.NoError) {
            revert("ECDSA: invalid signature");
        }
        return recovered;
    }

    /**
     * @dev Returns the address that signed a hashed message (`hash`) with
     * `signature` or error string. This address can then be used for verification purposes.
     */
    function tryRecover(bytes32 hash, bytes memory signature) internal pure returns (address, RecoverError) {
        if (signature.length == 65) {
            bytes32 r;
            bytes32 s;
            uint8 v;
            assembly {
                r := mload(add(signature, 0x20))
                s := mload(add(signature, 0x40))
                v := byte(0, mload(add(signature, 0x60)))
            }
            return tryRecover(hash, v, r, s);
        } else {
            return (address(0), RecoverError.InvalidSignatureLength);
        }
    }

    /**
     * @dev Overload of {ECDSA-tryRecover} that receives the `v`, `r` and `s` signature fields separately.
     */
    function tryRecover(bytes32 hash, uint8 v, bytes32 r, bytes32 s) internal pure returns (address, RecoverError) {
        // EIP-2 bit 0x7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a0 is the order of the curve divided by 2.
        // Non-canonical s-values are rejected to protect against signature malleability.
        if (uint256(s) > 0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5D576E7357A4501DDFE92F46681B20A0) {
            return (address(0), RecoverError.InvalidSignatureS);
        }

        // The v-value is usually 27 or 28.
        if (v != 27 && v != 28) {
            return (address(0), RecoverError.InvalidSignature);
        }

        // If the signature is valid (and not malleable), return the signer address
        address signer = ecrecover(hash, v, r, s);
        if (signer == address(0)) {
            return (address(0), RecoverError.InvalidSignature);
        }

        return (signer, RecoverError.NoError);
    }
}
