import path from "node:path";

import { exists } from "@ignored/hardhat-vnext-utils/fs";

export const ROOT_PATH_TO_FIXTURE: string = path.join(
  process.cwd(),
  "test",
  "fixture-projects",
  "cli",
  "telemetry",
);

export const TELEMETRY_FOLDER_PATH: string = path.join(
  process.cwd(),
  "src",
  "internal",
  "cli",
  "telemetry",
);

export async function checkIfSubprocessWasExecuted(
  resultFilePath: string,
): Promise<boolean> {
  // Checks if the subprocess was executed by waiting for a file to be created.
  // Uses an interval to periodically check for the file. If the file isn't found
  // within a specified number of attempts, an error is thrown, indicating a failure in subprocess execution.
  const MAX_COUNTER = 20;

  return new Promise((resolve, reject) => {
    let counter = 0;

    const intervalId = setInterval(async () => {
      counter++;

      if (await exists(resultFilePath)) {
        clearInterval(intervalId);
        resolve(true);
      } else if (counter > MAX_COUNTER) {
        clearInterval(intervalId);
        reject("Subprocess was not executed in the expected time");
      }
    }, 100);
  });
}