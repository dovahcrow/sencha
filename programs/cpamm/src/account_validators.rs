use crate::{
    Deposit, InitSwapToken, NewFactory, NewSwap, NewSwapMeta, Swap, SwapToken, SwapTokenInfo,
    SwapTokenWithFees, Withdraw,
};
use anchor_lang::{prelude::*, Key};
use vipers::validate::Validate;
use vipers::{assert_ata, assert_keys, invariant};

// --------------------------------
// Instruction account structs
// --------------------------------

impl<'info> Validate<'info> for NewFactory<'info> {
    fn validate(&self) -> ProgramResult {
        Ok(())
    }
}

impl<'info> Validate<'info> for NewSwap<'info> {
    fn validate(&self) -> ProgramResult {
        let pool_mint_decimals = self.token_0.mint.decimals.max(self.token_1.mint.decimals);

        // pool mint belongs to swap
        invariant!(
            self.pool_mint.decimals == pool_mint_decimals,
            "pool mint decimals must be the max of token A and token B mint"
        );
        assert_keys!(
            self.pool_mint.mint_authority.unwrap(),
            self.swap,
            "pool_mint.mint_authority"
        );
        assert_keys!(
            self.pool_mint.freeze_authority.unwrap(),
            self.swap,
            "pool_mint.freeze_authority"
        );

        // output_lp
        assert_keys!(self.output_lp.mint, *self.pool_mint, "output_lp.mint",);

        let token_0_mint = &self.token_0.mint;
        let token_1_mint = &self.token_1.mint;
        require!(
            token_0_mint.key() != token_1_mint.key(),
            SwapTokensCannotBeEqual
        );
        require!(token_0_mint.key() < token_1_mint.key(), SwapTokensNotSorted);

        let swap_key = self.swap.key();
        self.token_0.validate_for_swap(swap_key)?;
        self.token_1.validate_for_swap(swap_key)?;

        Ok(())
    }
}

impl<'info> Validate<'info> for NewSwapMeta<'info> {
    fn validate(&self) -> ProgramResult {
        // nothing to validate
        Ok(())
    }
}

impl<'info> Validate<'info> for Swap<'info> {
    fn validate(&self) -> ProgramResult {
        require!(!self.user.swap.is_paused, Paused);

        // inner validation will ensure that token source mint equals respective reserve
        let (swap_input, swap_output) =
            if self.input.reserve.key() == self.user.swap.token_0.reserves {
                (&self.user.swap.token_0, &self.user.swap.token_1)
            } else {
                (&self.user.swap.token_1, &self.user.swap.token_0)
            };
        self.input.validate_for_swap(swap_input)?;
        self.output.validate_for_swap(swap_output)?;

        Ok(())
    }
}

impl<'info> Validate<'info> for Withdraw<'info> {
    fn validate(&self) -> ProgramResult {
        require!(!self.user.swap.is_paused, Paused);

        assert_keys!(self.pool_mint, self.user.swap.pool_mint, "pool_mint");
        assert_keys!(self.input_lp.mint, self.pool_mint, "input_lp.mint");

        self.output_0.validate_for_swap(&self.user.swap.token_0)?;
        self.output_1.validate_for_swap(&self.user.swap.token_1)?;

        Ok(())
    }
}

impl<'info> Validate<'info> for Deposit<'info> {
    fn validate(&self) -> ProgramResult {
        require!(!self.user.swap.is_paused, Paused);

        // input_a, input_b should check their equal mints
        self.input_0.validate_for_swap(&self.user.swap.token_0)?;
        self.input_1.validate_for_swap(&self.user.swap.token_1)?;

        // should be same as swap
        assert_keys!(*self.pool_mint, self.user.swap.pool_mint, "pool_mint");

        // lp output destination
        assert_keys!(
            self.output_lp.mint,
            self.user.swap.pool_mint,
            "output_lp.mint"
        );
        invariant!(
            self.output_lp.owner != self.user.swap.key(),
            "output_lp.owner should not be the swap"
        );

        Ok(())
    }
}

// --------------------------------
// Account Structs
// --------------------------------

impl<'info> InitSwapToken<'info> {
    /// Validate the init swap.
    fn validate_for_swap(&self, swap: Pubkey) -> ProgramResult {
        // We could check token freeze authority presence
        // This ensures the swap will always be functional, since a freeze
        // would prevent the swap from working.
        // We do not think this is necessary to add.

        assert_keys!(self.fees.mint, *self.mint, "fees.mint");
        assert_keys!(self.fees.owner, swap, "fees.owner");
        assert_ata!(*self.reserve, swap, *self.mint, "reserve");

        // ensure the fee and reserve accounts are different
        // otherwise protocol fees would accrue to the LP holders
        invariant!(
            self.fees.key() != self.reserve.key(),
            "fees cannot equal reserve"
        );
        Ok(())
    }
}

impl<'info> SwapToken<'info> {
    fn validate_for_swap(&self, swap_info: &SwapTokenInfo) -> ProgramResult {
        assert_keys!(*self.reserve, swap_info.reserves, "reserve");
        assert_keys!(self.user.mint, swap_info.mint, "user.mint");

        // ensure no self-dealing
        invariant!(
            self.reserve.key() != self.user.key(),
            "user cannot be reserve account"
        );

        Ok(())
    }
}

impl<'info> SwapTokenWithFees<'info> {
    fn validate_for_swap(&self, swap_info: &SwapTokenInfo) -> ProgramResult {
        assert_keys!(*self.fees, swap_info.admin_fees, "fees");
        assert_keys!(*self.reserve, swap_info.reserves, "reserve");
        assert_keys!(self.user.mint, swap_info.mint, "user.mint");

        // ensure no self-dealing
        invariant!(
            self.fees.key() != self.user.key(),
            "user cannot be fees account"
        );
        invariant!(
            self.reserve.key() != self.user.key(),
            "user cannot be reserve account"
        );
        Ok(())
    }
}