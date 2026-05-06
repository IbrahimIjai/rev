// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ERC721} from "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import {ERC2771Context} from "@openzeppelin/contracts/metatx/ERC2771Context.sol";
import {Context} from "@openzeppelin/contracts/utils/Context.sol";

/**
 * @notice Gasless NFT — ERC-721 with ERC-2771 meta-transaction support.
 *
 * Users can mint and transfer NFTs without holding ETH. The gas relayer
 * submits transactions on their behalf; _msgSender() always returns the
 * real user address, never the relayer's.
 *
 * Deploy with the shared ERC2771Forwarder address; pass that same address
 * to the relayer as FORWARDER_ADDRESS.
 */
contract GaslessNFT is ERC721, ERC2771Context {
    uint256 private _nextTokenId;

    event Minted(address indexed to, uint256 indexed tokenId);

    constructor(
        string memory name_,
        string memory symbol_,
        address trustedForwarder
    ) ERC721(name_, symbol_) ERC2771Context(trustedForwarder) {}

    /// @notice Mint one NFT to the caller (gasless via forwarder).
    function mint() external returns (uint256 tokenId) {
        tokenId = _nextTokenId++;
        _safeMint(_msgSender(), tokenId);
        emit Minted(_msgSender(), tokenId);
    }

    /// @notice Gasless transfer — caller resolved via ERC-2771.
    function transfer(address to, uint256 tokenId) external {
        safeTransferFrom(_msgSender(), to, tokenId);
    }

    function totalMinted() external view returns (uint256) {
        return _nextTokenId;
    }

    // ERC721 + ERC2771Context both override _msgSender / _msgData — resolve here.
    function _msgSender() internal view override(Context, ERC2771Context) returns (address) {
        return ERC2771Context._msgSender();
    }

    function _msgData() internal view override(Context, ERC2771Context) returns (bytes calldata) {
        return ERC2771Context._msgData();
    }

    function _contextSuffixLength() internal view override(Context, ERC2771Context) returns (uint256) {
        return ERC2771Context._contextSuffixLength();
    }
}
