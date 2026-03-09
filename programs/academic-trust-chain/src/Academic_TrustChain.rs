use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

declare_id!("5WkgStN4hEMzvVYaqY7TLZwbQpCVaZLXtZgTcFSDi8sY");

#[program]
pub mod academic_trustchain {
    use super::*;

    pub fn issue_badge(ctx: Context<IssueBadge>, event_name: String) -> Result<()> {
        let clock = Clock::get()?;
        let achievement = &mut ctx.accounts.achievement_account;
        
        achievement.student = ctx.accounts.student.key();
        achievement.club = ctx.accounts.authority.key();
        achievement.event_name = event_name;
        achievement.timestamp = clock.unix_timestamp;

        msg!("Achievement issued for {}!", achievement.event_name);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(event_name: String)]
pub struct IssueBadge<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK: The recipient student
    pub student: UncheckedAccount<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 32 + 100 + 8,
        seeds = [b"achievement", student.key().as_ref(), event_name.as_bytes()],
        bump
    )]
    pub achievement_account: Account<'info, AchievementState>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct AchievementState {
    pub student: Pubkey,
    pub club: Pubkey,
    pub event_name: String,
    pub timestamp: i64,
}
