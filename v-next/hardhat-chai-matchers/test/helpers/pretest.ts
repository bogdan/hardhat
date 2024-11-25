import path, { dirname } from "node:path";
import { fileURLToPath } from "node:url";

import { createHardhatRuntimeEnvironment } from "@ignored/hardhat-vnext/hre";

// Compile the contracts before executing the tests to bypass the compilation step during testing.
(async () => {
  const currentDir = dirname(fileURLToPath(import.meta.url));

  const fixtureProjectDir = path.join(
    currentDir,
    "..",
    "fixture-projects",
    "hardhat-project",
  );

  const hre = await createHardhatRuntimeEnvironment({
    solidity: {
      version: "0.8.24", // Same version as the one in the contracts in the "hardhat-project" fixture project
    },
    paths: {
      artifacts: path.join(fixtureProjectDir, "artifacts"),
      sources: path.join(fixtureProjectDir, "contracts"),
    },
  });

  await hre.tasks.getTask("compile").run({});
})();
