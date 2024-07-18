use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use pyth_solana_receiver_sdk::price_update::{PriceUpdateV2, get_feed_id_from_hex};

declare_id!("FtDJTaT2Z7SECAWDS5KtXVFyzAD7ZDq73tjq5iWpmYpV");

// const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
// const USDT_MINT: &str = "BQcdHdAQW1hczDbBi9hiegXAR7A98Q9jx3X3iBBBDiq4";
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
    pub fn purchase_tokens_usd(ctx: Context<PurchaseTokensUSDContext>, amount: u64) -> Result<()> {
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

    pub fn purchase_tokens_sol(ctx: Context<PurchaseTokensSOLContext>, amount: u64) -> Result<()> {
        let presale_info = &mut ctx.accounts.presale_info;
        let stage_data = &mut ctx.accounts.stage_data;
        let price_update = &mut ctx.accounts.price_update;

        require!(presale_info.is_active, PresaleError::PresaleInactive);

        let index = presale_info.index;

        require_keys_eq!(presale_info.stage_data[index as usize], stage_data.key()); // provided pubkey for stage_data matches current stage

        let price = presale_info.stages[index as usize].price;
        let token_amount = presale_info.stages[index as usize].token_amount;

        let total_stage_amount = stage_data.total_stage_amount;
        
        // convert amount from SOL to USD
        let maximum_age: u64 = 3600;
        let feed_id: [u8; 32] = get_feed_id_from_hex("0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d")?;
        
        let usd_price = price_update.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;
        // decimals conversion
        let sol_price_in_usd: u64 = usd_price.price as u64;
        let sol_price_exponent: i32 = usd_price.exponent;

        let sol_price = (sol_price_in_usd as f64) * 10f64.powi(sol_price_exponent);
        let amount_usdc_f64 = price as f64 / 10f64.powi(6);
        let token_price_sol = amount_usdc_f64 / sol_price;
        // price for 1 token in SOL
        let token_price_sol_u64 = (token_price_sol * 10f64.powi(9)) as u64; // Convert SOL to lamports (1 SOL = 10^9 lamports)

        let mut purchased_amount = amount / token_price_sol_u64;

        if token_amount < (total_stage_amount + purchased_amount) {
            // if we have hitted max amount for current stage it should be set as inactive
            purchased_amount = token_amount - total_stage_amount;
            presale_info.is_active = false;
        }

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(), 
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.buyer.to_account_info().clone(),
                to: ctx.accounts.presale_account.clone(),
            });
        anchor_lang::system_program::transfer(cpi_context, token_amount * token_price_sol_u64)?;

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

    // only admin can call this function
    pub fn add_address_to_presale(ctx: Context<AddAddressContext>, address: Pubkey, amount: u64, usd_amount: u64) -> Result<()> {
        require_keys_eq!(ctx.accounts.admin.key(), ADMIN.parse::<Pubkey>().unwrap());

        let presale_info = &mut ctx.accounts.presale_info;
        let stage_data = &mut ctx.accounts.stage_data;

        // add address and purchased_amount to presale contract
        stage_data.purchase_records.push(PurchaseRecord {
            buyer: address,
            amount: amount,
        });    

        stage_data.total_stage_amount += amount;
        presale_info.total_supply += amount;
        presale_info.funds_raised += usd_amount;

        Ok(())
    }

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
pub struct PurchaseTokensUSDContext<'info> {
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

#[derive(Accounts)]
pub struct PurchaseTokensSOLContext<'info> {
    #[account(mut)]
    pub presale_info: Account<'info, PresaleInfo>,
    #[account(
        mut,
        realloc = 21 + 40 * (stage_data.purchase_records.len() + 1),
        realloc::payer = buyer,
        realloc::zero = true
    )]
    pub stage_data: Account<'info, StageData>,
    /// CHECK: this will be only address to send SOL to, so it safe
    #[account(mut)]
    pub presale_account: AccountInfo<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    pub price_update: Account<'info, PriceUpdateV2>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddAddressContext<'info> {
    #[account(mut)]
    pub presale_info: Account<'info, PresaleInfo>,
    #[account(
        mut,
        realloc = 21 + 40 * (stage_data.purchase_records.len() + 1),
        realloc::payer = admin,
        realloc::zero = true
    )]
    pub stage_data: Account<'info, StageData>,
    #[account(mut)]
    pub admin: Signer<'info>,
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
