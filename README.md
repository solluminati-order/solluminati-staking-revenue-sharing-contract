# SOLMI NFT epoch staking

Solluminati is offering to public their staking contract that is used for revenue sharing. Please enjoy and reach on our discord for feedback and possible other requests.
## Deploy on the devnet

Before build and test, you must install the node modules

```
npm i
```

### Build

```
anchor build
```

### Set to devent environment
```
solana config set --url devnet
```
Change the provider information on Anchor.toml
```
[provider]
cluster = "devnet"
wallet = "/home/john/.config/solana/id.json"
```

You must have at least 3 SOL in your wallet
Check the balance of wallet and airdrop 3 SOL
```
solana balance

solana airdrop 3
```

### Deploy to devnet
```
anchor deploy
```

Then you will get the program id after deploy successfully

You must change the program_id and declar_id on Anchor.toml and lib.rs

### Test

```
anchor test
```

You must modify the [provider] -> wallet on Anchor.toml


### What is Merkle?

We use a Merkle tree in order to whitelist an entire collection providing an array with all mints and creating a hash of the entire tree.
This hashed is than used in order to decode the tree and is stored in blockchain. The same Merkle tree is generated on FE side of the application,
that will provide a consistent way of whitelisting the NFTs that can be staked in this contract, because contract requires on the other interactions same
merkle proof both on FE and Contract side.

### Each contract functions can be called from the CLI project associated to this staking contract
CAN BE FOUND HERE => https://github.com/solluminati-order/solluminati-staking-revenue-sharing-cli

### The way it works?

The wallet that will be used for contract deploy will act like the BankWallet
In this wallet all the funds that you'd want to share with holders need to get here.

On initiliaze, the contract will generate a Treasury Address from where the rewards will be actually claimed by stakers.

### More details to be added soon. - How to use?!
