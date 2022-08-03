// use anchor_lang::solana_program::pubkey;
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

#[program]
pub mod coin_war {
    use super::*;

    const INITIAL_POOL_PRIZE: f64 = 100.00; 

    // Create a pool. This needs to be called once for each of the pools defined in enum Pools.
    pub fn create_pool(ctx: Context<CreatePool>, pool_name: String) -> Result<()> {
        require!(ctx.accounts.pool.is_initialized == false, ErrorCode::PoolAlreadyCreated);
        let clock: Clock = Clock::get().unwrap();
        let pool = &mut ctx.accounts.pool;
        pool.is_initialized = true;
        pool.last_update_timestamp = clock.unix_timestamp;
        pool.total_deposit = 0.00;
        pool.total_prize = INITIAL_POOL_PRIZE; // should be a non-zero number for cold-start
        pool.user_count = 0;
        pool.name = Pool::get_name(pool_name);

        Ok(())
    }

    // Create a game. This needs to be called once.
    pub fn create_game(ctx: Context<CreateGame>, start_time: i64, end_time: i64) -> Result<()> {
        let game = &mut ctx.accounts.game;
        game.start_time = start_time;
        game.end_time = end_time;
        game.winning_pool = String::from("solana");
        game.winning_amount = INITIAL_POOL_PRIZE;

        Ok(())
    }

    // Set every user average balance to the balance
    // Create new Game Account
    // pub fn startGame(ctx: Context<Game>) -> Result<()> {

    //     Ok(())
    // }

    // Tally up total for all the pools, and perform a weighted randomized selection for a winner
    // Calculate the total interests
    // Take 80% of total interest as the prize. Pick one winner for 4% of the prize
    // Distribute prize to the one big winner
    // Distribute prize to every other user in the winning pool, and record winnings for each user
    // Mark game as done and start next game  
    // pub fn endGame(ctx: Context<Game>) -> Result<()> {
        
    //     Ok(())
    // }

    // Transfer from user wallet to pool wallet
    // Update user balance
    // Update pool balance
    // Update pool count if needed
    // Update average balance for user
    // Create new transaction
    // Only allowed to deposit in one pool
    // pub fn withdraw(ctx: Context<Withdraw>, amount: f64) -> Result<()> {

    //     Ok(())
    // }

    // Transfer from pool wallet to user wallet
    // Update user balance
    // Update pool balance
    // Zero out average balance?
    pub fn deposit(ctx: Context<Deposit>, amount: f64) -> Result<()> {
        let clock: Clock = Clock::get().unwrap();

        // Transfer amount from user wallet to pool wallet
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info().clone(), // user wallet
            to: ctx.accounts.pool_token_account.to_account_info().clone(), // pool wallet
            authority: ctx.accounts.initializer.to_account_info().clone(),
        };
        let cpi_context = CpiContext::new(
            ctx.accounts.token_program.to_account_info().clone(), 
            cpi_accounts);

        token::transfer(cpi_context, amount as u64);

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
        if user.pool.trim().is_empty() || user.pool.ne(&ctx.accounts.pool.name) {
            user.pool = ctx.accounts.pool.name.clone();
        } 

        // Create new transaction
        let transaction = &mut ctx.accounts.transaction;
        transaction.amount = amount;
        transaction.transaction_type = Transaction::get_type("deposit".to_string());
        transaction.timestamp = clock.unix_timestamp;
        Ok(())
    }
}


#[derive(Accounts)]
pub struct StartGame<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>
}

#[derive(Accounts)]
pub struct EndGame<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>
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
    #[account(mut)]
    pub user: Account<'info, User>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(amount: f64)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(mut, seeds = [], bump)]
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
        seeds = ["Tx".as_ref(), user.key().as_ref(), pool.key().as_ref(), &user.txn_count.to_be_bytes()] 
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
    #[account(init, payer = initializer, space = User::LEN, seeds = ["User".as_ref(), initializer.key().as_ref()], bump)]
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

#[account]
pub struct Pool {
    pub is_initialized: bool,
    pub last_update_timestamp: i64,
    pub total_deposit: f64,
    pub total_prize: f64,
    pub user_count: u64,
    pub name: String,
}

#[account]
pub struct Game {
    pub start_time: i64,
    pub end_time: i64,
    pub winning_pool: String,
    pub winning_amount: f64,
}

// PDA
#[account]
pub struct Transaction {
    pub timestamp: i64,
    pub amount: f64,
    pub transaction_type: String,
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
    pub winning_pool: String,
    pub winning: f64
}

#[account]
pub struct User {
    pub balance: f64,
    // used to read UserGameHistory
    pub last_active: i64,
    pub game_history_count: u64,
    // needs to be reset to balance at the start of each game
    pub current_average_balance: f64,
    pub current_weighted_balance: f64,
    pub current_weighted_days: u64,
    pub txn_count: u64,
    pub pool: String,
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
        + STRING_PREFIX + POOL;
}
// Calculate space for Transaction Account
impl Transaction {
    const LEN: usize = DISCRIMINATOR
        + AMOUNT
        + TIMESTAMP 
        + STRING_PREFIX;

    fn get_type(key: String) -> String {
        let mut txn_types: HashMap<String, String> = HashMap::new();
        txn_types.insert(String::from("deposit"), String::from("deposit"));
        if txn_types.contains_key(&key) {
            return key;
        }
        return "".to_string();
    }
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

    fn get_name(key: String) -> String {
        let mut pool_names: HashMap<String, String> = HashMap::new();
        pool_names.insert("ethereum".to_string(), "Ethereum".to_string());
        pool_names.insert("bnb".to_string(), "BNB".to_string());
        pool_names.insert("solana".to_string(), "Solana".to_string());
        pool_names.insert("polygon".to_string(), "Polygon".to_string());
        if pool_names.contains_key(&key) {
            return key;
        }
        return "".to_string();
    }
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
}