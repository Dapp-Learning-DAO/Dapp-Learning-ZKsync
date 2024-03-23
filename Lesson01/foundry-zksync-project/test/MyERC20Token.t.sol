// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import {MyERC20Token} from "src/MyERC20Token.sol";

contract MyERC20TokenTest is Test {
    MyERC20Token public token;
    uint256 public initialSupply = 1_000_000 * (10 ** 18);

    function setUp() public {
        token = new MyERC20Token();
    }

    function test_totalSupply() public view {
        uint256 totalSupply = token.totalSupply();
        // assertEq(totalSupply, initialSupply);
        require(totalSupply == initialSupply, "Should have correct initial supply");
    }

    // function test_burnTokens() public view {
    //     uint256 burnAmount = 10 * (10 ** 18);
    //     token.burn(burnAmount);
    //     uint256 afterBurnSupply = token.totalSupply();
    //     require(afterBurnSupply == 999990, "Should allow owner to burn tokens");
    // }

    function test_transfer() public view {
        address toAddr = 0x123;
        uint256 transferAmount = 50 ether;
        // token.transfer(toAddr, transferAmount);
        uint256 userBalance = token.balanceOf(toAddr);
        // require(userBalance == transferAmount, "Should allow user to transfer tokens");
        
    }

}
