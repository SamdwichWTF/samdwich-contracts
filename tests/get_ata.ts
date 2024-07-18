import { getAssociatedTokenAddressSync } from '@solana/spl-token';
import { PublicKey } from '@solana/web3.js';

const OWNER = new PublicKey('BgCtoKRuamyHyWvsUiHHzGsUkaeHfDmXTBpxLrhwspBt');
const MINT = new PublicKey('37nuquRiY1fb55afNo2KqA9yMB9k7zj27Z6qXSjcsKxH');
const address2 = getAssociatedTokenAddressSync(MINT, OWNER);

console.log('Using SPL-Token: ', address2.toBase58());