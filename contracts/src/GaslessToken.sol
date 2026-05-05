// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ERC2771Context} from "@openzeppelin/contracts/metatx/ERC2771Context.sol";

/**
 * @notice Example token that supports gasless transfers via OZ ERC2771Forwarder.
 *
 * The forwarder appends the original sender's address to calldata; ERC2771Context
 * recovers it in _msgSender(), so transfer() sees the real user — never the relayer.
 *
 * Integration:
 *   1. Deploy OZ ERC2771Forwarder (shared per chain).
 *   2. Deploy this contract with that forwarder address as `trustedForwarder`.
 *   3. Users sign ForwardRequestData off-chain (EIP-712).
 *   4. Relayer calls forwarder.execute(ForwardRequestData).
 *   5. Forwarder calls transfer() here; _msgSender() == real user.
 */
contract GaslessToken is ERC2771Context {
    mapping(address => uint256) private _balances;
    uint256 private _totalSupply;
    string public name;
    string public symbol;
    uint8 public constant decimals = 18;

    event Transfer(address indexed from, address indexed to, uint256 value);

    constructor(
        string memory _name,
        string memory _symbol,
        address trustedForwarder
    ) ERC2771Context(trustedForwarder) {
        name = _name;
        symbol = _symbol;
    }

    function mint(address to, uint256 amount) external {
        require(to != address(0), "mint to zero address");
        _totalSupply += amount;
        _balances[to] += amount;
        emit Transfer(address(0), to, amount);
    }

    /// @dev _msgSender() from ERC2771Context returns the real user,
    ///      even when called through the trusted forwarder.
    function transfer(address to, uint256 amount) external returns (bool) {
        address sender = _msgSender();
        require(to != address(0), "transfer to zero address");
        require(_balances[sender] >= amount, "insufficient balance");
        _balances[sender] -= amount;
        _balances[to] += amount;
        emit Transfer(sender, to, amount);
        return true;
    }

    function balanceOf(address account) external view returns (uint256) {
        return _balances[account];
    }

    function totalSupply() external view returns (uint256) {
        return _totalSupply;
    }
}
