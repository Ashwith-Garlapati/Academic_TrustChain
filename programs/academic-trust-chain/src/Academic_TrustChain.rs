use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

declare_id!("5WkgStN4hEMzvVYaqY7TLZwbQpCVaZLXtZgTcFSDi8sY");

const MAX_NAME_LEN: usize = 100;
const MAX_URI_LEN:  usize = 200;
 
#[program]
pub mod academic_trustchain {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config         = &mut ctx.accounts.global_config;
        config.super_admin = ctx.accounts.authority.key();
        config.total_clubs = 0;
        config.bump        = ctx.bumps.global_config;
 
        msg!(
            "AcademicTrustChain initialized. Super admin: {}",
            config.super_admin
        );
        Ok(())
    }

    pub fn create_club(
        ctx:       Context<CreateClub>,
        club_name: String,
    ) -> Result<()> {
        require!(club_name.len() <= MAX_NAME_LEN, AcademicError::NameTooLong);
 
        let club          = &mut ctx.accounts.club_profile;
        club.president    = ctx.accounts.authority.key();
        club.club_name    = club_name.clone();
        club.is_verified  = false;
        club.total_events = 0;
        club.bump         = ctx.bumps.club_profile;
 
        let counter          = &mut ctx.accounts.badge_counter;
        counter.club         = ctx.accounts.club_profile.key();
        counter.total_issued = 0;
        counter.bump         = ctx.bumps.badge_counter;
 
        ctx.accounts.global_config.total_clubs += 1;
 
        msg!("Club '{}' created. Pending super admin verification.", club_name);
        Ok(())
    }

    pub fn verify_club(ctx: Context<VerifyClub>) -> Result<()> {
        require!(
            ctx.accounts.global_config.super_admin == ctx.accounts.authority.key(),
            AcademicError::Unauthorized
        );
        
        require!(
            !ctx.accounts.club_profile.is_verified,
            AcademicError::AlreadyVerified
        );
 
        ctx.accounts.club_profile.is_verified = true;
 
        msg!(
            "Club '{}' has been verified by super admin.",
            ctx.accounts.club_profile.club_name
        );
        Ok(())
    }

    pub fn create_event(
        ctx:          Context<CreateEvent>,
        event_name:   String,
        capacity:     u16,
        date:         i64,
        metadata_uri: String,
    ) -> Result<()> {
        require!(event_name.len()   <= MAX_NAME_LEN, AcademicError::NameTooLong);
        require!(metadata_uri.len() <= MAX_URI_LEN,  AcademicError::UriTooLong);
        require!(capacity > 0,                        AcademicError::InvalidCapacity);
        require!(
            ctx.accounts.club_profile.is_verified,
            AcademicError::ClubNotVerified
        );
        require!(
            ctx.accounts.club_profile.president == ctx.accounts.authority.key(),
            AcademicError::Unauthorized
        );
 
        let event_id            = ctx.accounts.club_profile.total_events + 1;
        let event               = &mut ctx.accounts.event_account;
        event.club              = ctx.accounts.club_profile.key();
        event.event_id          = event_id;
        event.event_name        = event_name.clone();
        event.capacity          = capacity;
        event.date              = date;
        event.metadata_uri      = metadata_uri;
        event.badges_issued     = 0;
        event.total_attended    = 0;
        event.checkin_open      = false;
        event.attendance_committed = false;
        event.attendance_hash   = [0u8; 32];
        event.is_active         = true;
        event.bump              = ctx.bumps.event_account;
 
        ctx.accounts.club_profile.total_events += 1;
 
        msg!(
            "Event '{}' (ID: {}) created with capacity {}.",
            event_name, event_id, capacity
        );
        Ok(())
    }

    pub fn open_checkin(ctx: Context<ManageCheckin>) -> Result<()> {
        require!(
            ctx.accounts.club_profile.president == ctx.accounts.authority.key(),
            AcademicError::Unauthorized
        );
        require!(
            ctx.accounts.event_account.is_active,
            AcademicError::EventNotActive
        );
        require!(
            !ctx.accounts.event_account.checkin_open,
            AcademicError::CheckinAlreadyOpen
        );
 
        ctx.accounts.event_account.checkin_open = true;
 
        msg!("Check-in opened for '{}'.", ctx.accounts.event_account.event_name);
        Ok(())
    }

    pub fn close_checkin(ctx: Context<ManageCheckin>) -> Result<()> {
        require!(
            ctx.accounts.club_profile.president == ctx.accounts.authority.key(),
            AcademicError::Unauthorized
        );
        require!(
            ctx.accounts.event_account.checkin_open,
            AcademicError::CheckinNotOpen
        );
 
        ctx.accounts.event_account.checkin_open = false;
 
        msg!(
            "Check-in closed for '{}'. Attended: {}.",
            ctx.accounts.event_account.event_name,
            ctx.accounts.event_account.total_attended
        );
        Ok(())
    }
 
    // --------------------------------------------------------
    // 6. COMMIT ATTENDANCE  — Hybrid Web2/Web3 trust bridge
    //
    //    After the event the backend:
    //      1. Collects all verified attendee wallet addresses
    //      2. Sorts the list alphabetically
    //      3. Computes SHA256(sorted_wallets.join(","))
    //      4. Calls this instruction with the hash + count
    //
    //    The hash is stored permanently inside EventRegistry.
    //    Anyone can audit the event by:
    //      - Downloading the attendance export from the backend
    //      - Recomputing SHA256(sorted list) locally
    //      - Comparing result to event_account.attendance_hash
    //
    //    Match  → list was never tampered with after the event
    //    No match → backend modified the list, provably caught
    //
    //    attendance_committed = true locks the hash so it
    //    cannot be overwritten after badge issuance begins.
    // --------------------------------------------------------
    pub fn commit_attendance(
        ctx:             Context<CommitAttendance>,
        attendance_hash: [u8; 32],
        total_attended:  u16,
    ) -> Result<()> {
        require!(
            ctx.accounts.club_profile.president == ctx.accounts.authority.key(),
            AcademicError::Unauthorized
        );
        require!(
            !ctx.accounts.event_account.checkin_open,
            AcademicError::CheckinStillOpen
        );
        require!(
            !ctx.accounts.event_account.attendance_committed,
            AcademicError::AttendanceAlreadyCommitted
        );
        require!(
            total_attended <= ctx.accounts.event_account.capacity,
            AcademicError::EventCapacityFull
        );
 
        ctx.accounts.event_account.attendance_hash      = attendance_hash;
        ctx.accounts.event_account.total_attended       = total_attended;
        ctx.accounts.event_account.attendance_committed = true;
 
        msg!(
            "Attendance committed for '{}'. {} attendees. Hash: {:?}",
            ctx.accounts.event_account.event_name,
            total_attended,
            attendance_hash
        );
        Ok(())
    }
 
    // --------------------------------------------------------
    // 7. ISSUE BADGE
    //    Club president issues a permanent on-chain badge to
    //    a student after attendance has been committed.
    //
    //    Requires:
    //      - Club verified by super admin
    //      - Check-in window is closed
    //      - Attendance hash has been committed on-chain
    //      - Event still has remaining capacity
    //
    //    The student wallet is UncheckedAccount because the
    //    Web2 backend already verified their attendance and
    //    the on-chain hash anchors the integrity of that list.
    //
    //    AchievementState PDA seeds [achievement + student +
    //    event] ensure the same badge cannot be issued twice —
    //    a second init attempt fails automatically.
    //
    //    event_name and metadata_uri are copied onto the badge
    //    at issue time so badge data is permanently frozen even
    //    if the EventRegistry is later updated or closed.
    // --------------------------------------------------------
    pub fn issue_badge(ctx: Context<IssueBadge>) -> Result<()> {
        require!(
            ctx.accounts.club_profile.is_verified,
            AcademicError::ClubNotVerified
        );
        require!(
            ctx.accounts.club_profile.president == ctx.accounts.authority.key(),
            AcademicError::Unauthorized
        );
        require!(
            ctx.accounts.event_account.is_active,
            AcademicError::EventNotActive
        );
        require!(
            !ctx.accounts.event_account.checkin_open,
            AcademicError::CheckinStillOpen
        );
        require!(
            ctx.accounts.event_account.attendance_committed,
            AcademicError::AttendanceNotCommitted
        );
        require!(
            ctx.accounts.event_account.badges_issued < ctx.accounts.event_account.capacity,
            AcademicError::EventCapacityFull
        );

        let clock                = Clock::get()?;
        let achievement          = &mut ctx.accounts.achievement_account;
        achievement.student      = ctx.accounts.student.key();
        achievement.club         = ctx.accounts.club_profile.key();
        achievement.event        = ctx.accounts.event_account.key();
        achievement.event_name   = ctx.accounts.event_account.event_name.clone();
        achievement.metadata_uri = ctx.accounts.event_account.metadata_uri.clone();
        achievement.timestamp    = clock.unix_timestamp;
        achievement.is_revoked   = false;
        achievement.bump         = ctx.bumps.achievement_account;

        ctx.accounts.event_account.badges_issued += 1;
        ctx.accounts.badge_counter.total_issued  += 1;

        msg!(
            "Badge issued to {} for '{}'.",
            ctx.accounts.student.key(),
            achievement.event_name
        );
        Ok(())
    }

    pub fn revoke_badge(ctx: Context<RevokeBadge>) -> Result<()> {
        require!(
            ctx.accounts.club_profile.president == ctx.accounts.authority.key(),
            AcademicError::Unauthorized
        );
        require!(
            ctx.accounts.achievement_account.club == ctx.accounts.club_profile.key(),
            AcademicError::Unauthorized
        );
        require!(
            !ctx.accounts.achievement_account.is_revoked,
            AcademicError::AlreadyRevoked
        );

        ctx.accounts.achievement_account.is_revoked     = true;
        ctx.accounts.event_account.badges_issued        -= 1;
        ctx.accounts.badge_counter.total_issued         -= 1;

        msg!(
            "Badge revoked for '{}'.",
            ctx.accounts.achievement_account.event_name
        );
        Ok(())
    }

    pub fn close_event(ctx: Context<CloseEvent>) -> Result<()> {
        require!(
            ctx.accounts.club_profile.president == ctx.accounts.authority.key(),
            AcademicError::Unauthorized
        );
        require!(
            ctx.accounts.event_account.is_active,
            AcademicError::EventNotActive
        );
 
        ctx.accounts.event_account.is_active    = false;
        ctx.accounts.event_account.checkin_open = false;
 
        msg!(
            "Event '{}' permanently closed. {}/{} badges issued.",
            ctx.accounts.event_account.event_name,
            ctx.accounts.event_account.badges_issued,
            ctx.accounts.event_account.capacity
        );
        Ok(())
    }
}

 
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
 
    #[account(
        init,
        payer  = authority,
        space  = GlobalConfig::LEN,
        seeds  = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
 
    pub system_program: Program<'info, System>,
}
 
 
#[derive(Accounts)]
#[instruction(club_name: String)]
pub struct CreateClub<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
 
    #[account(
        mut,
        seeds = [b"global_config"],
        bump  = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
 
    #[account(
        init,
        payer = authority,
        space = ClubRegistry::LEN,
        seeds = [b"club", authority.key().as_ref(), club_name.as_bytes()],
        bump
    )]
    pub club_profile: Account<'info, ClubRegistry>,

    // BadgeCounter is initialized alongside club in the same TX
    #[account(
        init,
        payer = authority,
        space = BadgeCounter::LEN,
        seeds = [b"badge_counter", club_profile.key().as_ref()],
        bump
    )]
    pub badge_counter: Account<'info, BadgeCounter>,
 
    pub system_program: Program<'info, System>,
}
 
 
#[derive(Accounts)]
pub struct VerifyClub<'info> {
    pub authority: Signer<'info>,
 
    #[account(
        seeds = [b"global_config"],
        bump  = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
 
    #[account(
        mut,
        seeds = [
            b"club",
            club_profile.president.as_ref(),
            club_profile.club_name.as_bytes()
        ],
        bump  = club_profile.bump
    )]
    pub club_profile: Account<'info, ClubRegistry>,
}
 
 
#[derive(Accounts)]
#[instruction(event_name: String, capacity: u16, date: i64, metadata_uri: String)]
pub struct CreateEvent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
 
    #[account(
        mut,
        seeds = [b"club", authority.key().as_ref(), club_profile.club_name.as_bytes()],
        bump  = club_profile.bump
    )]
    pub club_profile: Account<'info, ClubRegistry>,
 
    #[account(
        init,
        payer = authority,
        space = EventRegistry::LEN,
        seeds = [b"event", club_profile.key().as_ref(), event_name.as_bytes()],
        bump
    )]
    pub event_account: Account<'info, EventRegistry>,
 
    pub system_program: Program<'info, System>,
}
 
// Shared context for open_checkin and close_checkin.
// Both instructions need identical accounts so one struct serves
 
#[derive(Accounts)]
pub struct ManageCheckin<'info> {
    pub authority: Signer<'info>,
 
    #[account(
        seeds = [b"club", authority.key().as_ref(), club_profile.club_name.as_bytes()],
        bump  = club_profile.bump
    )]
    pub club_profile: Account<'info, ClubRegistry>,
 
    #[account(
        mut,
        seeds = [b"event", club_profile.key().as_ref(), event_account.event_name.as_bytes()],
        bump  = event_account.bump,
        constraint = event_account.club == club_profile.key() @ AcademicError::Unauthorized
    )]
    pub event_account: Account<'info, EventRegistry>,
}
 
 
#[derive(Accounts)]
pub struct CommitAttendance<'info> {
    pub authority: Signer<'info>,
 
    #[account(
        seeds = [b"club", authority.key().as_ref(), club_profile.club_name.as_bytes()],
        bump  = club_profile.bump
    )]
    pub club_profile: Account<'info, ClubRegistry>,
 
    #[account(
        mut,
        seeds = [b"event", club_profile.key().as_ref(), event_account.event_name.as_bytes()],
        bump  = event_account.bump,
        constraint = event_account.club == club_profile.key() @ AcademicError::Unauthorized
    )]
    pub event_account: Account<'info, EventRegistry>,
}
 
 
#[derive(Accounts)]
pub struct IssueBadge<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub student: UncheckedAccount<'info>,
 
    #[account(
        mut,
        seeds = [b"club", authority.key().as_ref(), club_profile.club_name.as_bytes()],
        bump  = club_profile.bump
    )]
    pub club_profile: Account<'info, ClubRegistry>,
 
    #[account(
        mut,
        seeds = [b"event", club_profile.key().as_ref(), event_account.event_name.as_bytes()],
        bump  = event_account.bump,
        constraint = event_account.club == club_profile.key() @ AcademicError::Unauthorized
    )]
    pub event_account: Account<'info, EventRegistry>,
 
    #[account(
        init,
        payer = authority,
        space = AchievementState::LEN,
        seeds = [b"achievement", student.key().as_ref(), event_account.key().as_ref()],
        bump
    )]
    pub achievement_account: Account<'info, AchievementState>,
 
    #[account(
        mut,
        seeds = [b"badge_counter", club_profile.key().as_ref()],
        bump  = badge_counter.bump
    )]
    pub badge_counter: Account<'info, BadgeCounter>,
 
    pub system_program: Program<'info, System>,
}
 
 
#[derive(Accounts)]
pub struct RevokeBadge<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [b"club", authority.key().as_ref(), club_profile.club_name.as_bytes()],
        bump  = club_profile.bump
    )]
    pub club_profile: Account<'info, ClubRegistry>,

    #[account(
        mut,
        seeds = [
            b"event",
            club_profile.key().as_ref(),
            achievement_account.event_name.as_bytes()
        ],
        bump  = event_account.bump,
        constraint = event_account.club == club_profile.key() @ AcademicError::Unauthorized
    )]
    pub event_account: Account<'info, EventRegistry>,

    #[account(
        mut,
        seeds = [
            b"achievement",
            achievement_account.student.as_ref(),
            achievement_account.event.as_ref()
        ],
        bump  = achievement_account.bump
    )]
    pub achievement_account: Account<'info, AchievementState>,

    #[account(
        mut,
        seeds = [b"badge_counter", club_profile.key().as_ref()],
        bump  = badge_counter.bump
    )]
    pub badge_counter: Account<'info, BadgeCounter>,
}
 
 
#[derive(Accounts)]
pub struct CloseEvent<'info> {
    pub authority: Signer<'info>,
 
    #[account(
        seeds = [b"club", authority.key().as_ref(), club_profile.club_name.as_bytes()],
        bump  = club_profile.bump
    )]
    pub club_profile: Account<'info, ClubRegistry>,
 
    #[account(
        mut,
        seeds = [b"event", club_profile.key().as_ref(), event_account.event_name.as_bytes()],
        bump  = event_account.bump,
        constraint = event_account.club == club_profile.key() @ AcademicError::Unauthorized
    )]
    pub event_account: Account<'info, EventRegistry>,
}

 
#[account]
pub struct GlobalConfig {
    pub super_admin:  Pubkey,
    pub total_clubs:  u64,
    pub bump:         u8,
}
impl GlobalConfig {
    pub const LEN: usize = 8 + 32 + 8 + 1;
}
 
 
#[account]
pub struct ClubRegistry {
    pub president:    Pubkey,
    pub club_name:    String,
    pub is_verified:  bool, 
    pub total_events: u64,  
    pub bump:         u8,   
}
impl ClubRegistry {
    pub const LEN: usize = 8 + 32 + (4 + MAX_NAME_LEN) + 1 + 8 + 1;
}
 
 
#[account]
pub struct EventRegistry {
    pub club:                 Pubkey,
    pub event_id:             u64,
    pub event_name:           String,
    pub capacity:             u16,
    pub date:                 i64,
    pub metadata_uri:         String,
    pub badges_issued:        u16,
    pub total_attended:       u16,
    pub checkin_open:         bool,
    pub attendance_committed: bool,
    pub attendance_hash:      [u8; 32],
    pub is_active:            bool,
    pub bump:                 u8,
}
impl EventRegistry {
    pub const LEN: usize = 8
        + 32
        + 8
        + (4 + MAX_NAME_LEN) 
        + 2
        + 8
        + (4 + MAX_URI_LEN)  
        + 2
        + 2
        + 1
        + 1
        + 32
        + 1
        + 1;
}
 
 
#[account]
pub struct AchievementState {
    pub student:      Pubkey,   
    pub club:         Pubkey,   
    pub event:        Pubkey,   
    pub event_name:   String,
    pub metadata_uri: String,
    pub timestamp:    i64,      
    pub is_revoked:   bool,     
    pub bump:         u8,       
}
impl AchievementState {
    pub const LEN: usize = 8
        + 32
        + 32
        + 32
        + (4 + MAX_NAME_LEN)
        + (4 + MAX_URI_LEN)
        + 8
        + 1
        + 1;                 
}
 
 
#[account]
pub struct BadgeCounter {
    pub club:         Pubkey,
    pub total_issued: u64,
    pub bump:         u8,
}
impl BadgeCounter {
    pub const LEN: usize = 8 + 32 + 8 + 1;
}

 
#[error_code]
pub enum AcademicError {
    #[msg("Name exceeds maximum allowed length")]
    NameTooLong,
    #[msg("URI exceeds maximum allowed length")]
    UriTooLong,
    #[msg("You are not authorized to perform this action")]
    Unauthorized,
    #[msg("Club has not been verified by the super admin")]
    ClubNotVerified,
    #[msg("Club is already verified")]
    AlreadyVerified,
    #[msg("Event is not active")]
    EventNotActive,
    #[msg("Check-in window is not open")]
    CheckinNotOpen,
    #[msg("Check-in window is already open")]
    CheckinAlreadyOpen,
    #[msg("Check-in is still open — close it before committing attendance")]
    CheckinStillOpen,
    #[msg("Attendance has already been committed for this event")]
    AttendanceAlreadyCommitted,
    #[msg("Attendance not committed yet — call commit_attendance first")]
    AttendanceNotCommitted,
    #[msg("Event has reached maximum capacity")]
    EventCapacityFull,
    #[msg("Badge has already been revoked")]
    AlreadyRevoked,
    #[msg("Capacity must be greater than zero")]
    InvalidCapacity,
}
 