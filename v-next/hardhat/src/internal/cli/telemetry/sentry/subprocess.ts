import { writeUtf8File } from "@ignored/hardhat-vnext-utils/fs";
import * as Sentry from "@sentry/node";
// import { captureMessage } from "@sentry/node";

import { Anonymizer } from "./anonymizer.js";
import { SENTRY_DSN } from "./reporter.js";

export async function main(): Promise<void> {
  try {
    Sentry.init({
      dsn: SENTRY_DSN,
    });
  } catch (error) {
    process.exit(1);
  }

  const serializedEvent = process.argv[2];
  const configPath = process.argv[3];

  if (serializedEvent === undefined) {
    await sendMsgToSentry(
      "There was an error parsing an event: 'process.argv[2]' argument is not set",
    );
    return;
  }

  let event: any;
  try {
    event = JSON.parse(serializedEvent);
  } catch {
    await sendMsgToSentry(
      "There was an error parsing an event: 'process.argv[2]' doesn't have a valid JSON",
    );
    return;
  }

  try {
    const anonymizer = new Anonymizer(configPath);
    const anonymizedEvent = await anonymizer.anonymize(event);

    if (anonymizedEvent.isRight()) {
      if (anonymizer.raisedByHardhat(anonymizedEvent.value)) {
        await sendEventToSentry(anonymizedEvent.value);
      }
    } else {
      await sendMsgToSentry(
        `There was an error anonymizing an event: ${anonymizedEvent.value}`,
      );
    }
  } catch (error: any) {
    await sendMsgToSentry(
      `There was an error capturing an event: ${error.message}`,
    );
    return;
  }
}

async function sendMsgToSentry(msg: string) {
  if (process.env.HARDHAT_TEST_SUBPROCESS_RESULT_PATH !== undefined) {
    // ATTENTION: only for testing
    await writeUtf8File(process.env.HARDHAT_TEST_SUBPROCESS_RESULT_PATH, msg);
    return;
  }

  //  captureMessage(msg);
}

async function sendEventToSentry(event: Sentry.Event) {
  if (process.env.HARDHAT_TEST_SUBPROCESS_RESULT_PATH !== undefined) {
    // ATTENTION: only for testing
    await writeUtf8File(
      process.env.HARDHAT_TEST_SUBPROCESS_RESULT_PATH,
      JSON.stringify(event),
    );
    return;
  }

  // Sentry.captureEvent(event);
}

await main();