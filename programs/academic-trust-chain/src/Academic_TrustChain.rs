use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

declare_id!("5WkgStN4hEMzvVYaqY7TLZwbQpCVaZLXtZgTcFSDi8sY");

#[program]
pub mod academic_trustchain {
    use super::*;

    pub fn issue_badge(ctx: Context<IssueBadge>, event_name: String) -> Result<()> {
        require!(event_name.len() <= 100, AcademicError::EventNameTooLong);

        let clock = Clock::get()?;
        let achievement = &mut ctx.accounts.achievement_account;

        achievement.student = ctx.accounts.student.key();
        achievement.event = ctx.accounts.authority.key();
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
        space = 8 + 32 + 32 + (4 + 100) + 8,
        seeds = [b"achievement", student.key().as_ref(), event_name.as_bytes()],
        bump
    )]
    pub achievement_account: Account<'info, AchievementState>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateClub<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + 32 + (4 + 100) + (4 + 100) + 1 + 1,
        seeds = [b"club", authority.key().as_ref()],
        bump
    )]
    pub club_profile: Account<'info, ClubRegistry>,

    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 8 + (4 + 100) + 1 + 8 + 1,
        seeds = [b"event", club.key().as_ref(), event_id.to_le_bytes().as_ref()],
        bump
    )]
    pub event_account: Account<'info, EventRegistry>

    pub system_program: Account<'info, System>,
}

#[account]
pub struct GlobalConfig {
    pub super_admin: Pubkey,
}

#[account]
pub struct AchievementState {
    pub student: Pubkey,
    pub event: Pubkey,
    pub event_name: String,
    pub timestamp: i64,
}

#[account]
pub struct ClubRegistry {
    pub president: Pubkey,
    pub club_name: String,
    pub university: String,
    pub is_verified: bool,
    pub bump: u8
}

#[account]
pub struct EventRegistry {
    pub club: Pubkey,
    pub event_id: u64,
    pub event_name: String,
    pub capacity: i8,
    pub date: i64,
    pun is_active: bool
}