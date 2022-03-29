import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { TokenStakeModel } from '../target/types/token_stake_model';
import { TOKEN_PROGRAM_ID, Token, ASSOCIATED_TOKEN_PROGRAM_ID, } from '@solana/spl-token';
import { assert } from "chai";
import invariant from "tiny-invariant";
import { PublicKey, SystemProgram, Transaction } from '@solana/web3.js';
import { BalanceTree } from "./balance-tree";
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';

describe('token-stake-model', () => {

  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenStakeModel as Program<TokenStakeModel>;

  let mintNFT = null; 
  
  let nft_vault_pda = null;
  let user_stake_pda = null;
  let nft_vault_bump = null;
  let user_stake_bump = null;
  let userNftTokenAccount = null;
  let nft_auth_pda = null;
  let nft_auth_bump = null;
  let merkle_pda = null;
  let merkle_bump = null;
  let epoch_state_pda = null;
  let epoch_state_bump = null;
  let stake_info_pda = null;
  let stake_info_bump = null;
  let stake_user_pda = null;
  let stake_user_bump = null;
  let treasury_pda = null;
  let treasury_bump = null;

  let leaves: {account: PublicKey}[] = [];
  let tree = null;
  let merkle_hash = null;
  let nftArray = [];

  const payer = anchor.web3.Keypair.generate();
  const nftAuthority = anchor.web3.Keypair.generate();
  const userAccount = anchor.web3.Keypair.generate();
  const bankAccount = anchor.web3.Keypair.generate();

  it('Get PDA', async () => {
    // Airdrop 3 SOL to payer
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer.publicKey, 3000000000),
      "confirmed"
    );

    await provider.send(
      (() => {
        const tx = new Transaction();
        tx.add(
          SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: bankAccount.publicKey,
            lamports: 1500000000,
          }),
        );
        tx.add(
          SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: userAccount.publicKey,
            lamports: 100000000,
          }),
        );
        return tx;
      })(),
      [payer]
    );

    // Get the authority of NFT
    [nft_auth_pda, nft_auth_bump] = await PublicKey.findProgramAddress([
      Buffer.from("vault-stake-auth"),
    ], program.programId);

    // Create mint nft address; decimal = 0
    mintNFT = await Token.createMint(
      provider.connection,
      payer,
      nftAuthority.publicKey,
      null,
      0,
      TOKEN_PROGRAM_ID,
    );

    // Create token account which can get the NFT
    userNftTokenAccount = await mintNFT.createAccount(userAccount.publicKey);

    // // Create the 1 NFT to user account
    await mintNFT.mintTo(
      userNftTokenAccount,
      nftAuthority.publicKey,
      [nftAuthority],
      1
    );

    // Get the pda for vault account which have NFT
    [nft_vault_pda, nft_vault_bump] = await PublicKey.findProgramAddress([
      Buffer.from("vault-stake"),
      mintNFT.publicKey.toBuffer(),
      userAccount.publicKey.toBuffer(),
    ], program.programId);

    // Get the account which have info of staking NFT
    [user_stake_pda, user_stake_bump] = await PublicKey.findProgramAddress([
      Buffer.from("user-stake"),
      mintNFT.publicKey.toBuffer(),
      userAccount.publicKey.toBuffer(),
    ], program.programId);

    [epoch_state_pda, epoch_state_bump] = await PublicKey.findProgramAddress([
      Buffer.from("epoch-state"),
    ], program.programId);

    [stake_info_pda, stake_info_bump] = await PublicKey.findProgramAddress([
      Buffer.from("stake-info"),
    ], program.programId);

    [treasury_pda, treasury_bump] = await PublicKey.findProgramAddress([
      Buffer.from("og-collection-treasury"),
    ], program.programId);

  });

  it('Is initialized!', async () => {

    await program.rpc.processInitialize(
      epoch_state_bump,
      stake_info_bump,
      treasury_bump,
      new anchor.BN(1000000000),
      {
        accounts: {
          bankAccount: bankAccount.publicKey,
          epochState: epoch_state_pda,
          stakeInfo: stake_info_pda,
          treasuryAccount: treasury_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        signers: [bankAccount]
      }
    );

  });

  it('Send Bonus to Treasury account', async () => {
    await program.rpc.processSendEpochBonus(
      epoch_state_bump,
      treasury_bump,
      new anchor.BN(500000000),
      {
        accounts: {
          bonusAccount: payer.publicKey,
          treasuryAccount: treasury_pda,
          epochState: epoch_state_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [payer]
      }
    );
  });

  it('Start new epoch', async () => {
    await program.rpc.processStartEpoch(
      epoch_state_bump,
      stake_info_bump,
      treasury_bump,
      {
        accounts: {
          bankAccount: bankAccount.publicKey,
          treasuryAccount: treasury_pda,
          epochState: epoch_state_pda,
          stakeInfo: stake_info_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [bankAccount]
      }
    );

  });

  it('Initialize Merkle tree!', async () => {
    [merkle_pda, merkle_bump] = await PublicKey.findProgramAddress([
      Buffer.from("Merkle"),
      payer.publicKey.toBuffer(),
    ], program.programId);

    const nft1 = anchor.web3.Keypair.generate();
    nftArray = [
      { account: nft1.publicKey},
      { account: mintNFT.publicKey},
    ];

    nftArray.map(x => leaves.push(x));
    tree = new BalanceTree(leaves);
    merkle_hash = tree.getRoot();
    
    
    await program.rpc.processInitializeMerkle(
      merkle_bump,
      toBytes32Array(merkle_hash),
      {
        accounts: {
          adminAccount: payer.publicKey,
          merkle: merkle_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [payer]
      }
    );
  });

  it('Initialize User!', async () => {
    [stake_user_pda, stake_user_bump] = await PublicKey.findProgramAddress([
      Buffer.from("stake_user"),
      userAccount.publicKey.toBuffer()
    ], program.programId);

    await program.rpc.processInitializeUser(
      stake_user_bump,
      {
        accounts: {
          userAccount: userAccount.publicKey,
          stakeUser: stake_user_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        signers: [userAccount]
      }
    );
  });

  it('Stake NFT', async () => {
    const proof = tree.getProof(nftArray[1]['account']);
    await program.rpc.processStakeNft(
      nft_vault_bump,
      nft_auth_bump,
      user_stake_bump,
      stake_info_bump,
      proof,
      {
        accounts: {
          userAccount: userAccount.publicKey,
          userNftTokenAccount: userNftTokenAccount,
          nftMint: mintNFT.publicKey,
          stakeInfoAccount: user_stake_pda,
          nftVaultAccount: nft_vault_pda,
          nftAuthority: nft_auth_pda,
          merkle: merkle_pda,
          stakeInfo: stake_info_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [userAccount]
      }
    );

    let _userNFTAccount = await mintNFT.getAccountInfo(userNftTokenAccount);
    assert.ok(_userNFTAccount.amount.toNumber() == 0);

    let _vault = await mintNFT.getAccountInfo(nft_vault_pda);
    assert.ok(_vault.amount.toNumber() == 1);
  });

  it('Unstake NFT', async () => {
    const proof = tree.getProof(nftArray[1]['account']);
    await program.rpc.processUnstakeNft(
      epoch_state_bump,
      stake_info_bump,
      proof,
      {
        accounts: {
          userAccount: userAccount.publicKey,
          userNftTokenAccount: userNftTokenAccount,
          nftMint: mintNFT.publicKey,
          nftVaultAccount: nft_vault_pda,
          stakeInfoAccount: user_stake_pda,
          vaultAuth: nft_auth_pda,
          merkle: merkle_pda,
          stakeUser: stake_user_pda,
          epochState: epoch_state_pda,
          stakeInfo: stake_info_pda,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [userAccount]
      }
    );

    let _userNFTAccount = await mintNFT.getAccountInfo(userNftTokenAccount);
    assert.ok(_userNFTAccount.amount.toNumber() == 1);

    let _vault = await mintNFT.getAccountInfo(nft_vault_pda);
    assert.ok(_vault.amount.toNumber() == 0);
  });

  it('Claim Reward token', async () => {

    await program.rpc.processClaimReward(
      treasury_bump,
      new anchor.BN(0),
      {
        accounts: {
          userAccount: userAccount.publicKey,
          treasuryAccount: treasury_pda,
          stakeUser: stake_user_pda,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [userAccount]
      }
    );
    
  });

  // it('Daily calculate reward', async () => {

  //   await program.rpc.processDailyReward(
  //     epoch_state_bump,
  //     stake_info_bump,
  //     {
  //       accounts: {
  //         stakeNftInfo: user_stake_pda,
  //         stakeUserAccount: stake_user_pda,
  //         stakeInfo: stake_info_pda,
  //         epochState: epoch_state_pda,
  //       },
  //     }
  //   );
    
  // });
});

const toBytes32Array = (b: Buffer): number[] => {
  invariant(b.length <= 32, `invalid length ${b.length}`);
  const buf = Buffer.alloc(32);
  b.copy(buf, 32 - b.length);

  return Array.from(buf);
};
