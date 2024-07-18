import { PublicKey } from '@solana/web3.js';

const OWNER = new PublicKey('BgCtoKRuamyHyWvsUiHHzGsUkaeHfDmXTBpxLrhwspBt');
const MINT = new PublicKey('37nuquRiY1fb55afNo2KqA9yMB9k7zj27Z6qXSjcsKxH');
const TOKEN_PROGRAM_ID = new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL');

const [address] = PublicKey.findProgramAddressSync(
    [OWNER.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), MINT.toBuffer()],
    ASSOCIATED_TOKEN_PROGRAM_ID
);

console.log('Using Solana-Web3.js: ', address.toBase58());