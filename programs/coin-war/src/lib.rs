use anchor_lang::solana_program::pubkey;
use solana_program::pubkey::Pubkey;
use anchor_lang::{prelude::*, solana_program};
use anchor_spl::token::{TokenAccount, Transfer, Token, Mint};
use anchor_spl::token;
use std::collections::HashMap;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
// TODO: Create pubkey for owner program and each of the four pool wallets
// const OWNER: Pubkey = pubkey!("adfadsfsdafssdfadsfdsaffdafssddyuoiwdafdsaf"); // TODO: this needs to be public key of the program wallet


/* Game Logic - User deposits USDC into one of 4 pools. At the end of the week selection of a winning pool based on weighted 
 * pool size happens. All users in the winning pool gets returns generated from all the other pools. One lucky winner in the 
 * winning pool will win 5% of the winning returns. 
 * Pool Logic - All deposits in each pool stays in the pool from one game to the next. 20% of returns generated from previous 
 * game stays in the pool as winnings.
 * Deposit/Withdrawal logic - During the game, any withdrawal will result in no winning for that game, while deposits will 
 * increase the average balance.
 */

// utility function to send tokens
fn transfer_token<'info>(
    user_sending: AccountInfo<'info>,
    user_receiving: AccountInfo<'info>,
    mint_of_token_being_sent: AccountInfo<'info>,
    escrow_wallet: &mut Account<'info, TokenAccount>,
    application_idx: u64,
    state: AccountInfo<'info>,
    state_bump: u8,
    token_program: AccountInfo<'info>,
    destination_wallet: AccountInfo<'info>,
    amount: u64
) -> Result<()> {
    let bump_vector = state_bump.to_le_bytes();
    let mint_of_token_being_sent_pk = mint_of_token_being_sent.key().clone();
    let application_idx_bytes = application_idx.to_le_bytes();
    let inner = vec![
        b"state".as_ref(),
        user_sending.key.as_ref(),
        user_receiving.key.as_ref(),
        mint_of_token_being_sent_pk.as_ref(), 
        application_idx_bytes.as_ref(),
        bump_vector.as_ref(),
    ];
    let outer = vec![inner.as_slice()];

    // Perform the actual transfer
    let transfer_instruction = Transfer{
        from: escrow_wallet.to_account_info(),
        to: destination_wallet,
        authority: state.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        transfer_instruction,
        outer.as_slice(),
    );
    anchor_spl::token::transfer(cpi_ctx, amount)?;
    Ok(())
}

#[program]
pub mod coin_war {
    use anchor_lang::accounts;

    use super::*;

    const INITIAL_POOL_PRIZE: f64 = 100.00; 
    const GAME_DURATION_IN_DAYS: i64 = 5;
    const GAME_DURATION_IN_SECS: i64 = GAME_DURATION_IN_DAYS * 24 * 60 * 60;

    // Create a pool. This needs to be called once for each of the pools defined in enum Pools.
    pub fn create_pool(ctx: Context<CreatePool>, pool_name: u8) -> Result<()> {
        let pool_enum = Pools::from(pool_name)?;
        require!(ctx.accounts.pool.is_initialized == false, ErrorCode::PoolAlreadyCreated);
        let clock: Clock = Clock::get().unwrap();
        let pool = &mut ctx.accounts.pool;
        pool.is_initialized = true;
        pool.last_update_timestamp = clock.unix_timestamp;
        pool.total_deposit = 0.00;
        pool.total_prize = INITIAL_POOL_PRIZE; // should be a non-zero number for cold-start
        pool.user_count = 0;
        pool.name = pool_enum.to_code();

        Ok(())
    }

    // Create a game. This needs to be called once.
    pub fn create_game(ctx: Context<CreateGame>, start_time: i64, end_time: i64, pool_name: u8) -> Result<()> {
        let pool = Pools::from(pool_name)?;
        let game = &mut ctx.accounts.game;
        game.start_time = start_time;
        game.end_time = end_time;
        game.winning_pool = pool.to_code();
        game.winning_amount = INITIAL_POOL_PRIZE;

        Ok(())
    }

    // Set every user average balance to the balance
    pub fn reset_user_average_balance(ctx: Context<ResetUserAverageBalance>) -> Result<()> {
        let user = &mut ctx.accounts.user;
        user.current_weighted_days = 7;
        user.current_weighted_balance = user.balance;
        user.current_average_balance = user.balance;
        Ok(())
    }

    // Create new Game Account
    pub fn start_game(ctx: Context<StartGame>, new_game_id: u64) -> Result<()> {
        let clock: Clock = Clock::get().unwrap();
        let game = &mut ctx.accounts.game;
        game.game_id = new_game_id;
        game.start_time = clock.unix_timestamp;
        game.end_time = clock.unix_timestamp + GAME_DURATION_IN_SECS;
        game.winning_amount = 0.0;
        game.winning_pool = 0;
        Ok(())
    }

    // Tally up total for all the pools, and perform a weighted randomized selection for a winner
    // Calculate the total interests
    // Take 80% of total interest as the prize. Pick one winner for 4% of the prize
    // Distribute prize to the one big winner
    // Distribute prize to every other user in the winning pool, and record winnings for each user
    // Mark game as done and start next game  
    pub fn end_game(ctx: Context<EndGame>, pool_names: Vec<u8>, pool_total: Vec<f64>) -> Result<()> {
        // consider moving everything other than the winner select off the blockchain
        // break into following methods: select_winning_pool(), calculate_interest(), select_winner_from_winning_pool(), 
        // pay_winner(), pay_winning_pool_user(), end_game()
        Ok(())
    }

    // Transfer from pool wallet to user wallet
    // Update user balance
    // Update pool balance
    // Update pool count if needed
    // Update average balance for user
    // Create new transaction
    // Only allowed to deposit in one pool
    pub fn withdraw(ctx: Context<Withdraw>, amount: f64, seedphrase: String) -> Result<()> {
        let clock: Clock = Clock::get().unwrap();

        // Transfer from pool wallet to user wallet
        let (_key, bump) = Pubkey::find_program_address(&[
            seedphrase.as_bytes()
            ], ctx.program_id);

        let signer_seed = [
            seedphrase.as_bytes(),
            &[bump]];

        // Transfer amount from pool wallet to user wallet
        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_token_account.to_account_info().clone(), // user wallet
            to: ctx.accounts.user_token_account.to_account_info().clone(), // pool wallet
            authority: ctx.accounts.initializer.to_account_info().clone(),
        };

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info().clone(), 
                cpi_accounts, 
                &[&signer_seed[..]]
            ),
            amount as u64
        )?;

        // Update user balance
        let user = &mut ctx.accounts.user;
        user.balance = user.balance - amount;

        // Update pool balance  
        let pool = &mut ctx.accounts.pool;
        pool.total_deposit = pool.total_deposit - amount;

        // Update pool count if needed
        if user.balance <= 0.0 {
            pool.user_count -= 1;
        }

        // Update average balance for user (user average balance is reset to current balance)
        user.current_average_balance = user.balance;
        user.current_weighted_balance = user.balance * GAME_DURATION_IN_DAYS as f64;
        user.current_weighted_days = GAME_DURATION_IN_DAYS;

        // Create new transaction
        let transaction = &mut ctx.accounts.transaction;
        transaction.amount = amount;
        transaction.transaction_type = Transaction_Type::Withdrawal.to_code();
        transaction.timestamp = clock.unix_timestamp;

        Ok(())
    }

    // Transfer from pool wallet to user wallet
    // Update user balance
    // Update pool balance
    // Zero out average balance?
    pub fn deposit(ctx: Context<Deposit>, amount: f64, seedphrase: String) -> Result<()> {
        let clock: Clock = Clock::get().unwrap();

        let (_key, bump) = Pubkey::find_program_address(&[
            seedphrase.as_bytes()
            ], ctx.program_id);

        let signer_seed = [
            seedphrase.as_bytes(),
            &[bump]];

        // Transfer amount from user wallet to pool wallet
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info().clone(), // user wallet
            to: ctx.accounts.pool_token_account.to_account_info().clone(), // pool wallet
            authority: ctx.accounts.initializer.to_account_info().clone(),
        };

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info().clone(), 
                cpi_accounts, 
                &[&signer_seed[..]]
            ),
            amount as u64
        )?;

        // Update user balance
        let user = &mut ctx.accounts.user;
        user.balance = user.balance + amount;

        // TODO: Update average balance for user
        let total_value = user.current_weighted_balance;
        let days_left = 4; // TODO: days left should be number of days left until game is over
        let added_value = days_left as f64 * amount;
        user.current_average_balance = (total_value + added_value) / (days_left as f64 + user.current_weighted_days as f64);
        user.current_weighted_balance = total_value + added_value;
        user.current_weighted_days = days_left + user.current_weighted_days;

        // Update pool balance
        let pool = &mut ctx.accounts.pool;
        pool.total_deposit = pool.total_deposit + amount;

        // Update pool count if user not in pool
        if user.pool == 0 || user.pool.ne(&pool.name) {
            user.pool = pool.name.clone();
            pool.user_count += 1;
        } 

        // Create new transaction
        let transaction = &mut ctx.accounts.transaction;
        transaction.amount = amount;
        transaction.transaction_type = Transaction_Type::Deposit.to_code();
        transaction.timestamp = clock.unix_timestamp;

        Ok(())
    }
}


#[derive(Accounts)]
#[instruction(new_game_id: u64)]
pub struct StartGame<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(init, payer = owner, space = Pool::LEN, seeds = [b"game".as_ref(), &new_game_id.to_be_bytes()], bump)]
    pub game: Account<'info, Game>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(game_id: u64)]
pub struct EndGame<'info> {
    // TODO: add constraint = owner.key() == OWNER
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"game".as_ref(), &game_id.to_be_bytes()], bump)]
    pub game: Account<'info, Game>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(start_time: i64, end_time: i64)]
pub struct CreateGame<'info> {
    // TODO: add constraint = owner.key() == OWNER
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(init, payer = owner, space = Game::LEN)]
    pub game: Account<'info, Game>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(amount: f64)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(mut, seeds = [b"user".as_ref(), initializer.key().as_ref()], bump)]
    pub user: Account<'info, User>,
    #[account(
        mut,
        token::mint = mint_address,
        token::authority = initializer.key(),
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    #[account(
        mut,
        token::mint = mint_address,
        token::authority = pool.key(),
    )]
    pub pool_token_account: Account<'info, TokenAccount>,   
    #[account(
        init, 
        payer = initializer, 
        space = Transaction::LEN, 
        seeds = [b"tx".as_ref(), user.key().as_ref(), pool.key().as_ref(), &user.txn_count.to_be_bytes()] 
        , bump)] 
    pub transaction: Account<'info, Transaction>,
    pub token_program: Program<'info, Token>,
    pub mint_address: Box<Account<'info, Mint>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(amount: f64)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(mut, seeds = [b"user".as_ref(), initializer.key().as_ref()], bump)]
    pub user: Account<'info, User>,
    #[account(
        mut,
        token::mint = mint_address,
        token::authority = initializer.key(),
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    #[account(
        mut,
        token::mint = mint_address,
        token::authority = pool.key(),
    )]
    pub pool_token_account: Account<'info, TokenAccount>,   
    #[account(
        init, 
        payer = initializer, 
        space = Transaction::LEN, 
        seeds = [b"tx".as_ref(), user.key().as_ref(), pool.key().as_ref(), &user.txn_count.to_be_bytes()] 
        , bump)] 
    pub transaction: Account<'info, Transaction>,
    pub token_program: Program<'info, Token>,
    pub mint_address: Box<Account<'info, Mint>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateUser<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(init, payer = initializer, space = User::LEN, seeds = [b"user".as_ref(), initializer.key().as_ref()], bump)]
    pub user: Account<'info, User>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ResetUserAverageBalance<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(init, payer = initializer, space = User::LEN, seeds = [b"user".as_ref(), initializer.key().as_ref()], bump)]
    pub user: Account<'info, User>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(pool_name: String)]
pub struct CreatePool<'info> {
    // TODO: add constraint = owner.key() == OWNER
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(init, payer = owner, space = Pool::LEN, seeds = [pool_name.as_ref()], bump)]
    pub pool: Account<'info, Pool>,
    pub pool_token_acccount: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Clone, Copy, PartialEq)]
enum Transaction_Type {
    Deposit,
    Withdrawal
}

impl Transaction_Type {
    fn to_code(&self) -> u8 {
        match self {
            Transaction_Type::Deposit => 1,
            Transaction_Type::Withdrawal => 2,
        }
    }

    fn from(val: u8) -> std::result::Result<Transaction_Type, Error> {
        match val {
            1 => Ok(Transaction_Type::Deposit),
            2 => Ok(Transaction_Type::Withdrawal),
            unknown_value => {
                msg!("Unknown transaction type: {}", unknown_value);
                Err(ErrorCode::TransactionTypeUnknown.into())
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Pools {
    Solana,
    BNB,
    Polygon,
    Ethereum
}

impl Pools {
    fn to_code(&self) -> u8 {
        match self {
            Pools::Solana => 1,
            Pools::BNB => 2,
            Pools::Polygon => 3,
            Pools::Ethereum => 4,
        }
    }

    fn from(val: u8) -> std::result::Result<Pools, Error> {
        match val {
            1 => Ok(Pools::Solana),
            2 => Ok(Pools::BNB),
            3 => Ok(Pools::Polygon),
            4 => Ok(Pools::Ethereum),
            unknown_value => {
                msg!("Unknown pool: {}", unknown_value);
                Err(ErrorCode::PoolUnknown.into())
            }
        }
    }
}

#[account]
pub struct Pool {
    pub is_initialized: bool,
    pub last_update_timestamp: i64,
    pub total_deposit: f64,
    pub total_prize: f64,
    pub user_count: u64,
    pub name: u8,
}

#[account]
pub struct Game {
    pub game_id: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub winning_pool: u8,
    pub winning_amount: f64,
}

#[account]
pub struct Transaction {
    pub timestamp: i64,
    pub amount: f64,
    pub transaction_type: u8,
}

#[account]
pub struct UserGameHistory {
    pub game_id: u64,
    pub winning: f64,
    pub user_id: u64,
}

#[account]
pub struct GameHistory {
    pub game_id: u64,
    pub winning_pool: u8,
    pub winning: f64
}

#[account]
pub struct User {
    pub pool: u8,
    pub balance: f64,
    // used to read UserGameHistory
    pub last_active: i64,
    pub game_history_count: u64,
    // needs to be reset to balance at the start of each game
    pub current_average_balance: f64,
    pub current_weighted_balance: f64,
    pub current_weighted_days: i64,
    pub txn_count: u64,
}

const DISCRIMINATOR: usize = 8;
const PUBLIC_KEY: usize = 32;
const TIMESTAMP: usize = 8;
const AMOUNT: usize = 8;
const COUNT: usize = 8;
const STRING_PREFIX: usize = 4; // Stores the size of the string
const POOL: usize = 20 * 4; // 20 chars max.
const U64: usize = 32;

// Calculate space for User Account
impl User {
    const LEN: usize = DISCRIMINATOR
        + AMOUNT
        + TIMESTAMP 
        + COUNT
        + AMOUNT
        + AMOUNT
        + AMOUNT
        + COUNT
        + STRING_PREFIX + POOL;
}
// Calculate space for Transaction Account
impl Transaction {
    const LEN: usize = DISCRIMINATOR
        + AMOUNT
        + TIMESTAMP 
        + STRING_PREFIX;
}
// TODO: Calculate space for UserGameHistory Account
// TODO: Calculate space for GameHistory Account

// Calculate space for Pool Account
impl Pool {
    const LEN: usize = DISCRIMINATOR
        + AMOUNT
        + TIMESTAMP 
        + AMOUNT
        + AMOUNT
        + COUNT;
}

// Calculate space for Game Account
impl Game {
    const LEN: usize = DISCRIMINATOR
        + TIMESTAMP
        + TIMESTAMP 
        + STRING_PREFIX + POOL
        + AMOUNT;
}

#[error_code]
pub enum ErrorCode {
    #[msg("You have no balance in the pool to withdraw.")]
    InvalidWithdrawal,
    #[msg("You can only contribute to one pool at a time.")]
    MultiplePoolNotAllowed,
    #[msg("This pool has already been created.")]
    PoolAlreadyCreated,
    #[msg("Wallet to withdraw from is not owned by owner.")]
    WalletToWithdrawFromInvalid,
    #[msg("Unknown transaction type.")]
    TransactionTypeUnknown,
    #[msg("Unknown pool.")]
    PoolUnknown,
}