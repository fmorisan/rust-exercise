# Rust Exercise
This is a toy transaction engine meant for completion of Cthulhu's challenge. 

## Usage
```
$ cargo run -- your_file.csv > output.csv
```

## Architecture
1. Transactions are read in through a CSV file
1. These transactions are parsed by `serde` and `csv` into the corresponding struct, then an enum to pattern-atch on.
1. Transaction objects are fed into the transaction engine one at a time in the same order they appear in the CSV (per spec, we can assume these happen chronologically)
1. Once all transactions are passed through the system, we extract the accounts from the engine and build report structs for each one (since the output format is not the same as the internal representation)

### Libraries used
* `serde` for serialization/deserialization
* `csv` for CSV formatting with `serde`
* `decimal_rs` for fixed-precision decimal math
* `clap` for command-line argument handling

### Engine internals
* The engine stores accounts in a BTreeMap so that account ordering at output time is stable.
* Account reporting and extraction happens through an iterator so we don't have to do the whole chunk at once.

### Scaling
* The `AccountState` struct may be placed behind a `Mutex` so that transactions are fed to it from a number of different streams at once. We just have to ensure only one transaction can be executed at any time.
* `AccountState` provides a getter for a specific account that may be used to peek into an account's balance. This may be used to get data to hydrate user-facing endpoints with.
* Ideally, the account state would not be in memory but on a database. Same logic for account operations applies, though.
* Accept transactions not only from CSV files but also from TCP streams or SQS messages.


## Considerations
Since the specification is ambiguous for some edge cases, I've taken the liberty to make some assumptions:
* We will **never** see a transaction ID twice with different data.
    * In this case, duplicates are not applied - results in an `Err(...)` from the accounts engine
* Disputes, Resolves and Chargebacks **MUST** be tagged with the original transaction's client.
    * It doesn't make sense that client 2 would be able to dispute client 1's deposit - these malformed transactions are ignored.
* Withdrawals may not be charged back.
    * From my understanding and intuition, it looks like `Deposit`s are initiated by the client themselves and may bne charged back through their credit card. `Withdrawal`s look like they are initiate by the platform.
* Accounts may hold negative balances
    * If a client disputes their deposit after withdrawing, the amount held on their account may go over their current balance. If they then chargeback this transaction, their balance will actually go negative.
    * This means that if their account would be unlocked in the future (which may happen in some other implementation) their starting balance would have to be covered first before continuing to operate.
* Once an account is locked, there's nothing more it may do
    * There's no way to unlock an account currently.


## Test coverage
```
$ cargo tarpaulin
```

```
running 86 tests
[...]
|| Uncovered Lines:
|| src/engine/state.rs: 47-48, 78
|| src/format/account.rs: 16, 19-22
|| src/format/transaction.rs: 70
|| src/main.rs: 19-25, 28, 30, 32, 35, 37-39
|| Tested/Total Lines:
|| src/engine/account.rs: 41/41 +0.00%
|| src/engine/state.rs: 63/66 +0.00%
|| src/engine/transaction.rs: 23/23 +0.00%
|| src/format/account.rs: 0/5 +0.00%
|| src/format/transaction.rs: 10/11 +0.00%
|| src/main.rs: 0/14 +0.00%
|| 
85.62% coverage, 137/160 lines covered, +0.00% change in coverage
```

## Details

Given a CSV representing a series of transactions, implement a simple toy transactions engine that processes the payments crediting and debiting accounts. After processing the complete set of payments output the client account balances.

The input file is the first and only argument to the binary. Output should be written to stdout.


### Input

For example:

```
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
```

### Output

| Column | Description |
|---|---|
| available | The total funds that are available for trading, staking, withdrawal, etc. This should be equal to the total - held amounts |
| held | The total funds that are held for dispute. This should be equal to total - available amounts |
| total | The total funds that are available or held. This should be equal to available + held |
| locked | Whether the account is locked. An account is locked if a charge back occurs |

```
client, available, held, total, locked
1, 1.5, 0.0, 1.5, false
2, 2.0, 0.0, 2.0, false
```
