# Affiliate Swap Contract

The Affiliate Swap contract is a smart contract for executing a swap between two
tokens with affiliate fees. The contract supports swapping tokens using Osmosis.

The contract charges an affiliate fee for each swap transaction, which is
deducted from the swap amount and transferred to the provided fee collector
address. The affiliate fee percentage is configurable and has a default value of
5%.

## Usage

### Contract Functions

#### Instantiation

- `instantiate`: Initializes the contract by storing the max fee percentage and version information for future migrations.

    **Messages**

``` rust
pub struct InstantiateMsg {
    pub max_fee_percentage: Option<Decimal>,
}
```

When instantiating the json message would look like:

``` json
{
  "max_fee_percentage": "5.5"
}
```

The max fee cannot be larger than 50%

#### Execution

- `swap`: Executes a swap and charges the affiliate fee. It takes the following input parameters:

    - `routes`: An array of `SwapAmountInRoute` structs specifying the input and output tokens along with the pool in which to execute the swap.
    - `token_out_min_amount`: The minimum amount of output token expected to receive from the swap.
    - `fee_percentage`: The percentage of the swap amount charged as an affiliate fee. If not provided, the default value of 5% is used.
    - `fee_collector`: The address to which the affiliate fee is transferred.

    **Messages**

``` rust
pub enum ExecMsg {
    Swap {
        routes: Vec<SwapAmountInRoute>,
        token_out_min_amount: Coin,
        fee_percentage: Option<Decimal>,
        fee_collector: String,
    },
}

// The actual implementation of SwapAmountInRoute is in osmosis_std 
pub struct SwapAmountInRoute {
    pub pool_id: u64,
    pub token_out_denom: String,
}
```

As an example, when calling the contract you can use a message like:

``` json
{
  "swap": {
    "routes": [
      {
        "pool_id": 1,
        "token_out_denom": "uosmo"
      },
      {
        "pool_id": 2,
        "token_out_denom": "uion"
      }
    ],
    "token_out_min_amount": {
      "denom": "uion",
      "amount": "1000000"
    },
    "fee_percentage": "0.5",
    "fee_collector": "osmo1exampleaddr"
  }
}
```


#### Queries

- `get_max_fee_percentage`: Retrieves the max fee percentage stored in the contract.

### Responses

#### MaxFeePercentage query response

```rust
#[cw_serde]
pub struct MaxFeePercentageResponse {
    pub max_fee_percentage: Decimal,
}
```

- `max_fee_percentage`: The maximum affiliate fee percentage that can be charged on a swap transaction.

#### Swap execute response

- `original_sender`: The address of the user who initiated the swap.
- `fee`: The amount of affiliate fee charged on the swap transaction.
- `fee_collector`: The address to which the affiliate fee is transferred.
- `swap_in_denom`: The denomination of the input token.
- `swap_in_amount`: The amount of input token provided for the swap.
- `token_out_denom`: The denomination of the output token received

```rust
#[cw_serde]
pub struct SwapResponse {
    pub original_sender: String,
    pub fee: Uint128,
    pub fee_collector: Addr,
    pub swap_in_denom: String,
    pub swap_in_amount: Uint128,
    pub token_out_denom: String,
    pub token_out_amount: Uint128,
}
```

### Events

The contract emits one event when a swap is executed successfully:

- `affiliate_swap`: This event indicates that a swap has been executed and includes the following attributes:
  - `sender`: The address of the original sender who initiated the swap.
  - `swap_token_in`: The amount and denomination of the token that was swapped into the contract.
  - `fee`: The amount and denomination of the fee that was charged for the swap.
  - `token_out`: The amount and denomination of the token that was received as a result of the swap.

These events can be used by external systems to track the activity of the
contract, as well as to generate reports and analytics.


