use solana_program::pubkey::Pubkey;
use anchor_lang::{prelude::*, solana_program};
use anchor_spl::token::{TokenAccount, Transfer, Token, Mint};
use anchor_spl::token;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

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

    // Create all the pools listed in the Pools enum
    pub fn createPool(ctx: Context<CreatePool>, pool_name: String) -> Result<()> {
        require!(ctx.accounts.pool.isInitialized == false, ErrorCode::PoolAlreadyCreated);
        ctx.accounts.pool.isInitialized = true;
        
        Ok(())
    }

    // Set every user average balance to the balance
    // Create new Game Account
    pub fn startGame(ctx: Context<Game>) -> Result<()> {
        Ok(())
    }

    // Tally up total for all the pools, and perform a weighted randomized selection for a winner
    // Calculate the total interests
    // Take 80% of total interest as the prize. Pick one winner for 4% of the prize
    // Distribute prize to the one big winner
    // Distribute prize to every other user in the winning pool, and record winnings for each user
    // Mark game as done and start next game  
    pub fn endGame(ctx: Context<Game>) -> Result<()> {
        
        Ok(())
    }

    // Transfer from user wallet to pool wallet
    // Update user balance
    // Update pool balance
    // Update average balance for user
    // Only allowed to deposit in one pool
    pub fn deposit(ctx: Context<Withdraw>) -> Result<()> {
        Ok(())
    }

    // Transfer from pool wallet to user wallet
    // Update user balance
    // Update pool balance
    // Zero out average balance?
    pub fn withdraw(ctx: Context<Deposit>) -> Result<()> {
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
    #[account(init, payer = initializer, space = User::LEN)]
    pub user: Account<'info, User>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    #[account(
        init, 
        payer = initializer, 
        space = Transaction::LEN, 
        seeds = ["Tx".as_ref(), user.key().as_ref(), pool.key().as_ref(), &user.txn_count.to_be_bytes()] 
        , bump)]
    pub transaction: Account<'info, Transaction>,
    pub system_program: Program<'info, System>,
}

const OWNER: Pubkey = static_pubkey::static_pubkey!("jakcasdfasfaskcb"); // public key of the program wallet
#[derive(Accounts)]
#[instruction(pool_name: String)]
pub struct CreatePool<'info> {
    #[account(mut, constraint = owner.key() == OWNER)]
    pub owner: Signer<'info>,
    #[account(init, payer = owner, space = Pool::LEN, seeds = [pool_name.as_ref()], bump)]
    pub pool: Account<'info, Pool>,
    pub system_program: Program<'info, System>,
}

enum TransactionType {
    Deposit,
    Withdrawal,
}

enum Pools {
    Etherum,
    Bnb,
    Solana,
    Polygon,
}

#[account]
pub struct Pool {
    pub isInitialized: bool,
    pub lastUpdateTimestamp: i64,
    pub totalDeposit: f64,
    pub totalPrize: f64,
    pub user_count: u64,
}

#[account]
pub struct Game {
    pub startTime: u64,
    pub endTime: u64,
    pub winningPool: String,
    pub winningAmount: f64,
}

// PDA
#[account]
pub struct Transaction {
    pub timestamp: i64,
    pub amount: f64,
    pub transaction_type: u64,
}

#[account]
pub struct UserGameHistory {
    pub gameId: u64,
    pub winning: f64,
    pub userId: u64,
}

#[account]
pub struct GameHistory {
    pub gameId: u64,
    pub winningPool: String,
    pub winning: f64
}

#[account]
pub struct User {
    pub balance: f64,
    // used to read UserGameHistory
    pub lastActive: i64,
    pub game_history_count: u64,
    // needs to be reset to balance at the start of each game
    pub current_average_balance: f64,
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

#[error_code]
pub enum ErrorCode {
    #[msg("You have no balance in the pool to withdraw.")]
    InvalidWithdrawal,
    #[msg("You can only contribute to one pool at a time.")]
    MultiplePoolNotAllowed,
    #[msg("This pool has already been created.")]
    PoolAlreadyCreated,
}