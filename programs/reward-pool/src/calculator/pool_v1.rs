use crate::calculator::*;

pub struct RewardCalculatorV1;

impl RewardCalculator for RewardCalculatorV1 {
    fn reward_per_token(
        &self,
        pool: &Account<Pool>,
        total_staked: u64,
        last_time_reward_applicable: u64,
    ) -> (u128, u128) {
        if total_staked == 0 {
            return (
                pool.reward_a_per_token_stored,
                pool.reward_b_per_token_stored,
            );
        }

        let a = pool
            .reward_a_per_token_stored
            .checked_add(
                (last_time_reward_applicable as u128)
                    .checked_sub(pool.last_update_time as u128)
                    .unwrap()
                    .checked_mul(pool.reward_a_rate as u128)
                    .unwrap()
                    .checked_mul(PRECISION)
                    .unwrap()
                    .checked_div(total_staked as u128)
                    .unwrap(),
            )
            .unwrap();

        let b = pool
            .reward_b_per_token_stored
            .checked_add(
                (last_time_reward_applicable as u128)
                    .checked_sub(pool.last_update_time as u128)
                    .unwrap()
                    .checked_mul(pool.reward_b_rate as u128)
                    .unwrap()
                    .checked_mul(PRECISION)
                    .unwrap()
                    .checked_div(total_staked as u128)
                    .unwrap(),
            )
            .unwrap();

        (a, b)
    }

    fn rate_after_funding(
        &self, 
        pool: &mut Account<Pool>, 
        reward_a_vault: &Account<TokenAccount>, 
        reward_b_vault: &Account<TokenAccount>, 
        funding_amount_a: u64, 
        funding_amount_b: u64) -> Result<(u64, u64)> {

        //a little inception here, a pool V1 funding needs to handle the upgrade of the pool
        //to V2.  However at the same time it needs to handle the reason that pool V2 exists
        //which is to fix a bug that caused some funds to get stuck and not emit.

        //rescuing borked funds
        //V1 farms calculated rate using lamports per second resulting in farms with a rate of 0
        //but token in the vault.  These tokens never emitted anything and won't get picked up unless
        //the rate is updated based on the *vault contents*, not the computed emissions.
        //As such, we add the vault contents to the funding amount.

        pool.upgrade_if_needed();

        let mut funding_amount_a = funding_amount_a;
        let mut funding_amount_b = funding_amount_b;

        if pool.reward_a_rate == 0                  //are not emitting
            && pool.reward_a_per_token_stored == 0  //never owed anyone anything
            && reward_a_vault.amount > 0            //yet the fault has funds
        {
            funding_amount_a = funding_amount_a.checked_add(reward_a_vault.amount).unwrap();
        }

        if pool.reward_b_rate == 0                  //are not emitting
            && pool.reward_b_per_token_stored == 0  //never owed anyone anything
            && reward_b_vault.amount > 0            //yet the fault has funds
        {
            funding_amount_b = funding_amount_b.checked_add(reward_b_vault.amount).unwrap();
        }

        //now get the latest calc for the pool and use it
        let calc = get_calculator(pool);
        calc.rate_after_funding(pool, reward_a_vault, reward_b_vault, funding_amount_a, funding_amount_b)
        
        /*
        let current_time = clock::Clock::get()
            .unwrap()
            .unix_timestamp
            .try_into()
            .unwrap();
        let reward_period_end = pool.reward_duration_end;

        let a: u64;
        let b: u64;

        if current_time >= reward_period_end {
            a = funding_amount_a.checked_div(pool.reward_duration).unwrap();
            b = funding_amount_b.checked_div(pool.reward_duration).unwrap();
        } else {
            let remaining = pool.reward_duration_end.checked_sub(current_time).unwrap();
            let leftover_a = remaining.checked_mul(pool.reward_a_rate).unwrap();
            let leftover_b = remaining.checked_mul(pool.reward_b_rate).unwrap();

            a = funding_amount_a
                .checked_add(leftover_a)
                .unwrap()
                .checked_div(pool.reward_duration)
                .unwrap();
            b = funding_amount_b
                .checked_add(leftover_b)
                .unwrap()
                .checked_div(pool.reward_duration)
                .unwrap();
        }

        (a, b)
        */
    }

    fn user_earned_amount(
        &self,
        pool: &anchor_lang::Account<Pool>,
        user: &anchor_lang::Account<User>,
    ) -> (u64, u64) {

        let a: u64 = (user.balance_staked as u128)
            .checked_mul(
                (pool.reward_a_per_token_stored as u128)
                    .checked_sub(user.reward_a_per_token_complete as u128)
                    .unwrap(),
            )
            .unwrap()
            .checked_div(PRECISION)
            .unwrap()
            .checked_add(user.reward_a_per_token_pending as u128)
            .unwrap()
            .try_into()
            .unwrap();

        let b: u64 = (user.balance_staked as u128)
            .checked_mul(
                (pool.reward_b_per_token_stored as u128)
                    .checked_sub(user.reward_b_per_token_complete as u128)
                    .unwrap(),
            )
            .unwrap()
            .checked_div(PRECISION)
            .unwrap()
            .checked_add(user.reward_b_per_token_pending as u128)
            .unwrap()
            .try_into()
            .unwrap();
            
        (a, b)
    }
}
