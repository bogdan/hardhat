import type { HardhatRuntimeEnvironmentHooks } from "@ignored/hardhat-vnext/types/hooks";

import { NetworkManagerImplementation } from "../network-manager.js";

export default async (): Promise<Partial<HardhatRuntimeEnvironmentHooks>> => ({
  created: async (context, hre) => {
    hre.network = new NetworkManagerImplementation(
      hre.globalOptions.network !== ""
        ? hre.globalOptions.network
        : hre.config.defaultNetwork,
      hre.config.defaultChainType,
      hre.config.networks,
      context.hooks,
    );
  },
});
