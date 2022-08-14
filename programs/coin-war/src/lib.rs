use anchor_spl::associated_token::AssociatedToken;
use solana_program::pubkey::Pubkey;
use anchor_lang::{prelude::*, solana_program};
use anchor_spl::token::{TokenAccount, Transfer, Token, Mint};
use anchor_spl::token;

declare_id!("9X5F3QKnsgsJyLe1hAco6XRxk9CYGt91mKuUAFgd1ihY");
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


fn to_long<'info>(amount: f64) -> u64 {
    let mint_decimals = 9;
    return (amount * f64::powf(10., mint_decimals.into())) as u64;
}

// utility function to send tokens out of pool wallets
fn transfer_token_out_of_pool<'info>(
    pool_wallet: &mut Account<'info, TokenAccount>,
    token_program: AccountInfo<'info>,
    destination_wallet: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    pool_name: String,
    amount: u64
) -> Result<()> {
    let inner = vec![b"pool_wallet".as_ref()];
    let outer = vec![inner.as_slice()];

    // Perform the actual transfer
    let transfer_instruction = Transfer{
        from: pool_wallet.to_account_info(),
        to: destination_wallet,
        authority: authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        transfer_instruction,
        outer.as_slice(),
    );
    return anchor_spl::token::transfer(cpi_ctx, amount);
}

#[program]
pub mod coin_war {
    use std::vec;

    use super::*;

    const GAME_DURATION_IN_DAYS: i64 = 5;
    const MINIMUM_DEPOSIT: f64 = 1.00;
    // const INITIAL_POOL_PRIZE: f64 = 100.00; 
    // const GAME_DURATION_IN_SECS: i64 = GAME_DURATION_IN_DAYS * 24 * 60 * 60;
    // const JACKPOT_WINNER_PERCENTAGE: u64 = 10;

    // Create a pool. This needs to be called once for each of the pools defined in enum Pools.
    pub fn create_pool(ctx: Context<CreatePool>, pool_name: u8) -> Result<()> {
        let pool_enum = Pools::from(pool_name)?;
        require!(ctx.accounts.pool.is_initialized == false, ErrorCode::PoolAlreadyCreated);
        let clock: Clock = Clock::get().unwrap();
        let pool = &mut ctx.accounts.pool;
        pool.is_initialized = true;
        pool.last_update_timestamp = clock.unix_timestamp;
        pool.total_deposit = 0.00;
        pool.user_count = 0;
        pool.name = pool_enum.to_code();
        pool.average_prediction = 0.0;

        Ok(())
    }

    pub fn create_user(ctx: Context<CreateUser>) -> Result<()> {
        let user = &mut ctx.accounts.user;
        user.balance = 0.0;
        user.current_average_balance = 0.0;
        user.current_weighted_balance = 0.0;
        user.current_weighted_days = GAME_DURATION_IN_DAYS;
        user.last_prediction = 0.0;

        Ok(())
    }

    // Tally up total for all the pools, pick the pool with the average prediction closest to the actual prediction
    // Calculate the total prize (interest)
    // Take 80% of total interest as the prize. Pick one winner for 10% of the prize
    // Distribute prize to the one big winner
    // Distribute rest of 75% prize to every other user in the winning pool, and record winnings for each user
    // Mark game as done and start next game  

    // consider moving everything other than the winner select off the blockchain
    // break into following methods: select_winning_pool(), process_interest(), select_winner_from_winning_pool(), 
    // pay_winner(), pay_winning_pool_user(), end_game()

    // Perform weighted randomized selection out of the 4 pools
    pub fn select_winning_pool(ctx: Context<SelectWinningPool>, pool_names: Vec<u8>, pool_predictions: Vec<f64>, pool_coin_prices: Vec<f64>) -> Result<String> {
        // check to see if the parameters are correct
        require!(pool_names.len() == pool_predictions.len(), ErrorCode::PoolsDataSizeDoNotMatch);
        require!(pool_predictions.len() == pool_coin_prices.len(), ErrorCode::PoolsDataSizeDoNotMatch);
        require!(Pools::Solana.to_code() == pool_names[0], ErrorCode::PoolsInWrongOrder);
        require!(Pools::BNB.to_code() == pool_names[1], ErrorCode::PoolsInWrongOrder);
        require!(Pools::Polygon.to_code() == pool_names[2], ErrorCode::PoolsInWrongOrder);
        require!(Pools::Ethereum.to_code() == pool_names[3], ErrorCode::PoolsInWrongOrder);

        // generate random winnning pool represented by number from 0 - 99
        // let time_stamp = &mut ctx.accounts.clock.unix_timestamp.clone();
        // let time_stamp_string = time_stamp.to_string();
        // let time_stamp_chars = &mut time_stamp_string.chars();
        // let first_digit = time_stamp_chars.last().unwrap();
        // let second_digit = time_stamp_chars.nth(time_stamp_chars.clone().count() - 2).unwrap();
        // let digits_array = [first_digit, second_digit];
        // let s: String = digits_array.iter().collect();
        // let winning_pool_random: u64 = s.parse().unwrap();
        // let winning_pool_index = winning_pool_random + 1;        

        // we divide the slots by the order defined in Pools enum Solana, BNB, Polygon, Ethereum with relative weights
        // let total: f64 = pool_total.iter().sum();
        // let mut pool_weights: Vec<u64> = Vec::new();
        // for i in 0..pool_total.len() {
        //     let mut running_total: f64 = 0.0;
        //     let pool_weight = pool_total[i] / total;
        //     running_total += pool_weight;
        //     pool_weights.push(running_total as u64);
        // }
        // let mut winning_index = 0;
        // for j in 0..pool_weights.len() {
        //     if winning_pool_index <= pool_weights[j] {
        //         winning_index = j;
        //         break;
        //     }
        // }

        // choose the pool_predictions with the smallest % delta to pool_coin_prices and mark as winning index
        let mut winning_index = 0;
        let mut current_smallest_delta = 100000.000;
        for i in 0..pool_predictions.len() {
            let delta = (pool_predictions[i] - pool_coin_prices[i]).abs();
            if delta < current_smallest_delta {
                current_smallest_delta = delta;
                winning_index = i;
            }
        }    

        winning_index += 1;
        let winning_pool: String = Pools::code_to_string(winning_index as u8);

        Ok(winning_pool)
    }   

    // Calculate percent of the pool the user balance represents and pay out according
    // Takes in one user at a time
    pub fn pay_winning_pool_user(ctx: Context<PayWinner>, user_key: Pubkey, pool_name: String, prize_amount: f64) -> Result<()> {
        let total_deposit = ctx.accounts.pool.total_deposit.clone();
        let user_balance = ctx.accounts.user.balance.clone();
        let percentage_of_pool = user_balance / total_deposit;
        let prize = percentage_of_pool * prize_amount;

        let result = transfer_token_out_of_pool(
            &mut ctx.accounts.pool_token_account, 
            ctx.accounts.token_program.to_account_info(), 
            ctx.accounts.user_token_account.to_account_info(), 
            ctx.accounts.owner.to_account_info(), 
            pool_name, 
            prize as u64);

        if result.is_ok() {
            // reset user balances
            let user = &mut ctx.accounts.user;
            user.current_average_balance = user.balance.clone();
            user.current_weighted_balance = user.balance.clone();
            user.current_weighted_days = GAME_DURATION_IN_DAYS;
            user.last_prediction = 0.0;
        }

        Ok(())
    }
    
    // Allow user to update prediction (especially when a new game starts)
    pub fn make_prediction(ctx: Context<MakePrediction>, prediction: f64) -> Result<()> {
        let user = &mut ctx.accounts.user;
        let pool = &mut ctx.accounts.pool;
        
        // Remove previous prediction and update
        let total_prediction = pool.average_prediction * pool.user_count as f64;

        let new_total_prediction = total_prediction - user.last_prediction + prediction;
        user.last_prediction = prediction;
        pool.average_prediction = new_total_prediction / pool.user_count as f64;

        Ok(())
    }

    // Transfer from pool wallet to user wallet
    // Update user balance
    // Update pool balance
    // Update prediction
    // Update pool count if needed
    // Update average balance for user
    // Create new transaction
    // Only allowed to deposit in one pool
    pub fn withdraw(ctx: Context<Withdraw>, amount: f64) -> Result<()> {
        let clock: Clock = Clock::get().unwrap();
        let pool = &mut ctx.accounts.pool;
        let pool_number = pool.name;
        let pool_name : String = Pools::code_to_string(pool_number);

        // Check if theres enough money
        let user_balance = ctx.accounts.user.balance.clone();
        require!(user_balance < amount, ErrorCode::InsufficientBalance); 

        let result = transfer_token_out_of_pool(
            &mut ctx.accounts.pool_token_account, 
            ctx.accounts.token_program.to_account_info(), 
            ctx.accounts.user_token_account.to_account_info(), 
            ctx.accounts.initializer.to_account_info(), 
            pool_name, 
            amount as u64);

        require!(result.is_ok(), ErrorCode::PaymentFailed);

        // Update user balance
        let user = &mut ctx.accounts.user;
        user.balance = user.balance - amount;

        // Update pool balance  
        let pool = &mut ctx.accounts.pool;
        pool.total_deposit = pool.total_deposit - amount;

        // Remove previous prediction and update
        let total_prediction = pool.average_prediction * pool.user_count as f64;

        // Update pool count if needed
        if user.balance <= 0.0 {
            pool.user_count -= 1;
        }

        let new_total_prediction = total_prediction - user.last_prediction;
        user.last_prediction = 0.0;
        pool.average_prediction = new_total_prediction / pool.user_count as f64;

        // Update average balance for user (user average balance is reset to current balance)
        user.current_average_balance = user.balance;
        user.current_weighted_balance = user.balance * GAME_DURATION_IN_DAYS as f64;
        user.current_weighted_days = GAME_DURATION_IN_DAYS;

        // Create new transaction
        let transaction = &mut ctx.accounts.transaction;
        transaction.amount = amount;
        transaction.transaction_type = TransactionType::Withdrawal.to_code();
        transaction.timestamp = clock.unix_timestamp;

        Ok(())
    }

    // Transfer from pool wallet to user wallet
    // Update user balance
    // Update prediction
    // Update pool balance
    // Zero out average balance?
    pub fn deposit(ctx: Context<Deposit>, amount: f64, prediction: f64) -> Result<()> {
        require!(amount >= MINIMUM_DEPOSIT, ErrorCode::DepositInsufficient);
        let clock: Clock = Clock::get().unwrap();
        let user = &mut ctx.accounts.user;
        let key = user.key();

        let inner = vec![
            b"user_wallet".as_ref(),
            key.as_ref(),
        ];
        let outer = vec![inner.as_slice()];

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
                outer.as_slice(),
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

        
        // Remove previous prediction and update
        let total_prediction = pool.average_prediction * pool.user_count as f64;

        // Update pool count if user not in pool
        if user.pool == 0 || user.pool.ne(&pool.name) {
            user.pool = pool.name.clone();
            pool.user_count += 1;
        }

        let new_total_prediction = total_prediction - user.last_prediction + prediction;
        user.last_prediction = prediction;
        pool.average_prediction = new_total_prediction / pool.user_count as f64;

        // Create new transaction
        // let transaction = &mut ctx.accounts.transaction;
        // transaction.amount = amount;
        // transaction.transaction_type = TransactionType::Deposit.to_code();
        // transaction.timestamp = clock.unix_timestamp;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(amount: f64, pool_name: String)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(mut, seeds = [b"user".as_ref(), initializer.key().as_ref()], bump)]
    pub user: Account<'info, User>,
    #[account(
        constraint=user_token_account.owner == user.key(),
        constraint=user_token_account.mint == mint_address.key(),
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    #[account(
        mut,
        seeds = [b"pool_wallet".as_ref(), pool_name.as_ref()],
        bump,
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
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(amount: f64, pool_name: String, prediction: f64)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(mut, seeds = [b"user".as_ref(), initializer.key().as_ref()], bump)]
    pub user: Account<'info, User>,
    #[account(
        mut,
        constraint=user_token_account.owner == user.key(),
        constraint=user_token_account.mint == mint_address.key(),
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut, seeds = [pool_name.as_ref()], bump)]
    pub pool: Account<'info, Pool>,
    #[account(
        mut,
        constraint=pool_token_account.owner == pool.key(),
        constraint=pool_token_account.mint == mint_address.key(),
        seeds = [b"pool_wallet".as_ref()],
        bump,
    )]
    pub pool_token_account: Account<'info, TokenAccount>,   
    pub token_program: Program<'info, Token>,
    pub mint_address: Box<Account<'info, Mint>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(user_key: Pubkey, pool_name: String, prize_amount: f64)]
pub struct PayWinner<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"user".as_ref(), user_key.as_ref()], bump)]
    pub user: Account<'info, User>,
    #[account(
        mut,
        seeds = [b"user_wallet".as_ref(), user_key.as_ref()],
        bump
    )]
    pub user_token_account: Account<'info, TokenAccount>,   
    #[account(mut, seeds = [pool_name.as_ref()], bump)]
    pub pool: Account<'info, Pool>,
    #[account(
        mut,
        constraint=pool_token_account.owner == owner.key(),
        constraint=pool_token_account.mint == mint_address.key(),
        seeds = [b"pool_wallet".as_ref()],
        bump,
    )]
    pub pool_token_account: Account<'info, TokenAccount>,  
    pub token_program: Program<'info, Token>,
    pub mint_address: Box<Account<'info, Mint>>,
    pub system_program: Program<'info, System>, 
}

#[derive(Accounts)]
#[instruction(pool_name: String, prediction: f64)]
pub struct MakePrediction<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"user".as_ref(), owner.key().as_ref()], bump)]
    pub user: Account<'info, User>,
    #[account(mut, seeds = [pool_name.as_ref()], bump)]
    pub pool: Account<'info, Pool>,
}

#[derive(Accounts)]
#[instruction(pool_names: Vec<u8>, pool_total: Vec<f64>)]
pub struct SelectWinningPool<'info> {
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct CreateUser<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,
    #[account(
        init, 
        payer = initializer, 
        space = User::LEN, 
        seeds = [b"user".as_ref(), initializer.key().as_ref()], 
        bump)]
    pub user: Account<'info, User>,
    #[account(
        init,
        payer = initializer,
        seeds = [b"user_wallet".as_ref(), user.key().as_ref()],
        bump,
        token::mint = mint_address,
        token::authority = user,
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub mint_address: Box<Account<'info, Mint>>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(pool_name: String)]
pub struct CreatePool<'info> {
    // TODO: add constraint = owner.key() == OWNER
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(init, payer = owner, space = Pool::LEN, seeds = [pool_name.as_ref()], bump)]
    pub pool: Account<'info, Pool>,
    #[account(
        init,
        payer = owner,
        seeds = [b"pool_wallet".as_ref()],
        bump,
        token::mint = mint_address,
        token::authority = pool,
    )]
    pub pool_token_account: Account<'info, TokenAccount>,
    pub mint_address: Box<Account<'info, Mint>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Clone, Copy, PartialEq)]
enum TransactionType{
    Deposit,
    Withdrawal
}

impl TransactionType {
    fn to_code(&self) -> u8 {
        match self {
            TransactionType::Deposit => 1,
            TransactionType::Withdrawal => 2,
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

    fn code_to_string(val: u8) -> String {
        match val {
            1 => "Solana".to_string(),
            2 => "BNB".to_string(),
            3 => "Polygon".to_string(),
            4 => "Ethereum".to_string(),
            _ => "".to_string(),
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
    pub user_count: u64,
    pub name: u8,
    pub average_prediction: f64,
}

#[account]
pub struct Game {
    pub game_id: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub winning_pool: u8,
    pub winning_amount: f64,
    pub total_prize: f64,
}

#[account]
pub struct Transaction {
    pub timestamp: i64,
    pub amount: f64,
    pub transaction_type: u8,
}

#[account]
pub struct User {
    pub pool: u8,
    pub last_prediction: f64,
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
const TIMESTAMP: usize = 8;
const AMOUNT: usize = 8;
const COUNT: usize = 8;
const STRING_PREFIX: usize = 4; // Stores the size of the string
const POOL: usize = 20 * 4; // 20 chars max.

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
    #[msg("You have insufficient balance for this withdrawal.")]
    InsufficientBalance,
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
    #[msg("Payment failed.")]
    PaymentFailed,
    #[msg("Pools in wrong order.")]
    PoolsInWrongOrder,
    #[msg("Pool data sizes do not match.")]
    PoolsDataSizeDoNotMatch,
    #[msg("Minimum Deposit amount is 1 sol.")]
    DepositInsufficient,
}