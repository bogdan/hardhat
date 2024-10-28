import type { HardhatPlugin } from "@ignored/hardhat-vnext/types/plugins";

import { task } from "@ignored/hardhat-vnext/config";

import "./type-extensions.js";

const hardhatPlugin: HardhatPlugin = {
  id: "test",
  tasks: [
    task("test", "Runs tests using the Mocha test runner")
      .addVariadicArgument({
        name: "testFiles",
        description: "An optional list of files to test",
        defaultValue: [],
      })
      .addFlag({
        name: "bail",
        description: "Stop running tests after the first test failure",
      })
      .addOption({
        name: "grep",
        description: "Only run tests matching the given string or regexp",
        defaultValue: "",
      })
      .setAction(import.meta.resolve("./task-action.js"))
      .build(),
  ],
  hookHandlers: {
    config: import.meta.resolve("./hookHandlers/config.js"),
  },
};

export default hardhatPlugin;
