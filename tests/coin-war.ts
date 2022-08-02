import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { CoinWar } from "../target/types/coin_war";

describe("coin-war", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.CoinWar as Program<CoinWar>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
