use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("HNtzKW53K3Dx4vNUCuGrHqsDGY8w6RiXqrxnh7BxRfGJ");

// main net beta addresses
// const _USDC_MINT: sol_pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
// const _USDT_MINT: sol_pubkey = pubkey!("BQcdHdAQW1hczDbBi9hiegXAR7A98Q9jx3X3iBBBDiq4");
const ADMIN: &str = "7oBnRhq4SWr71CnMwr4U9SVcoLhrKMdALDxv9Kbq8RME";

#[program]
pub mod presale {
    use super::*;

    pub fn initialize(ctx: Context<InitializeContext>) -> Result<()> {
        require_keys_eq!(ctx.accounts.admin.key(), ADMIN.parse::<Pubkey>().unwrap());

        let presale_info = &mut ctx.accounts.presale_info;
        let stage_data = &mut ctx.accounts.stage_data;

        presale_info.index = 0;
        let index: usize = presale_info.index.into();
        presale_info.is_active = true;
        presale_info.phase_start_time = Clock::get()?.unix_timestamp;
        presale_info.funds_raised = 0;
        presale_info.total_supply = 0;
        presale_info.stages = [
            PresaleStage {
                token_amount: 4_444_444_444_44,
                price: 30,
            },
            PresaleStage {
                token_amount: 8_888_888_888_88,
                price: 31,
            },
            PresaleStage {
                token_amount: 17_777_777_777_76,
                price: 32,
            },
            PresaleStage {
                token_amount: 44_444_444_444_40,
                price: 33,
            },
            PresaleStage {
                token_amount: 88_888_888_888_80,
                price: 34,
            },
            PresaleStage {
                token_amount: 186_666_666_666_48,
                price: 35,
            },
        ];

        stage_data.stage_num = 1;
        stage_data.total_stage_amount = 0;

        presale_info.stage_data[index] = stage_data.key();

        Ok(())
    }

    pub fn start_next_stage(ctx: Context<StartNextStageContext>) -> Result<()> {
        require_keys_eq!(ctx.accounts.admin.key(), ADMIN.parse::<Pubkey>().unwrap());

        let presale_info = &mut ctx.accounts.presale_info;
        let stage_data = &mut ctx.accounts.stage_data;

        // TODO: uncomment this for production!
        // require!(!presale_info.is_active, PresaleError::PreviousStageActive);

        require!(presale_info.index < 6, PresaleError::InvalidStage);

        presale_info.is_active = true;
        presale_info.index += 1;
        let index: usize = presale_info.index.into();

        presale_info.phase_start_time = Clock::get()?.unix_timestamp;

        stage_data.stage_num = (index as u8) + 1;
        stage_data.total_stage_amount = 0;
        presale_info.stage_data[index] = stage_data.key();

        Ok(())
    }

    // amount will be in usdc or usdt (or sol -> handle separately)
    pub fn purchase_tokens_usd(ctx: Context<BuyTokens>, amount: u64) -> Result<()> {
        let presale_info = &mut ctx.accounts.presale_info;
        let stage_data = &mut ctx.accounts.stage_data;

        require!(presale_info.is_active, PresaleError::PresaleInactive);

        let index = presale_info.index;

        require_keys_eq!(presale_info.stage_data[index as usize], stage_data.key()); // provided pubkey for stage_data matches current stage

        let price = presale_info.stages[index as usize].price;
        let token_amount = presale_info.stages[index as usize].token_amount;

        let total_stage_amount = stage_data.total_stage_amount;

        let mut purchased_amount = amount / price;

        if token_amount < (total_stage_amount + purchased_amount) {
            // if we have hitted max amount for current stage it should be set as inactive
            purchased_amount = token_amount - total_stage_amount;
            presale_info.is_active = false;
        }

        // transfer amount of tokens to multisig wallet and update presale data for buyer and amount
        let cpi_accounts = Transfer {
            from: ctx.accounts.buyer_token_account.to_account_info().clone(),
            to: ctx.accounts.presale_token_account.to_account_info().clone(),
            authority: ctx.accounts.buyer.to_account_info().clone(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, purchased_amount * price)?;

        // add address and purchased_amount to presale contract
        stage_data.purchase_records.push(PurchaseRecord {
            buyer: ctx.accounts.buyer.key(),
            amount: purchased_amount,
        });

        stage_data.total_stage_amount += purchased_amount;
        presale_info.total_supply += purchased_amount;
        presale_info.funds_raised += purchased_amount * price;

        Ok(())
    }

    // pub purchase_tokens_sol(ctx: Context<PurchaseTokensSOL>, stage: u8, amount: u64) -> Result<()> {
    //     Ok(())
    // }

}

#[derive(Accounts)]
pub struct InitializeContext<'info> {
    #[account(init, payer = admin, space = 8 + PresaleInfo::INIT_SPACE)]
    pub presale_info: Account<'info, PresaleInfo>,
    #[account(init, payer = admin, space = 8 + StageData::INIT_SPACE)]
    pub stage_data: Account<'info, StageData>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StartNextStageContext<'info> {
    #[account(mut)]
    pub presale_info: Account<'info, PresaleInfo>,
    #[account(init, payer = admin, space = 8 + StageData::INIT_SPACE)]
    pub stage_data: Account<'info, StageData>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyTokens<'info> {
    #[account(mut)]
    pub presale_info: Account<'info, PresaleInfo>,
    #[account(
        mut,
        realloc = 21 + 40 * (stage_data.purchase_records.len() + 1),
        realloc::payer = buyer,
        realloc::zero = true
    )]
    pub stage_data: Account<'info, StageData>,
    #[account(mut)]
    pub presale_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct PresaleInfo {
    pub index: u8,
    pub is_active: bool,
    pub phase_start_time: i64,
    pub funds_raised: u64,
    pub total_supply: u64,
    pub stages: [PresaleStage; 6],
    pub stage_data: [Pubkey; 6],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct PresaleStage {
    pub token_amount: u64,
    pub price: u64,
}

// space calculation 1 + 8 + size of a vector ( 4 + (size of 1 element) * (number of elements))
#[account]
#[derive(InitSpace)]
pub struct StageData {
    pub stage_num: u8,
    pub total_stage_amount: u64,
    #[max_len(1)]
    pub purchase_records: Vec<PurchaseRecord>,
}

// One record takes 32 + 8 = 40 bytes
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct PurchaseRecord {
    pub buyer: Pubkey,
    pub amount: u64,
}

#[error_code]
pub enum PresaleError {
    #[msg("Previous stage is still active")]
    PreviousStageActive,
    #[msg("Presale is not active")]
    PresaleInactive,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Invalid stage")]
    InvalidStage,
    #[msg("Maximum raise exceeded")]
    MaxRaiseExceeded,
}
