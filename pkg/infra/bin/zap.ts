#!/usr/bin/env node
import * as cdk from "aws-cdk-lib";
import { ZapStack } from "../lib/zap_stack";

// AWS account and region.
const env = {
  account: process.env.CDK_DEFAULT_ACCOUNT,
  region: process.env.CDK_DEFAULT_REGION,
};

// Error if the account ID is unfamiliar.
// This is a sanity-check to make sure I don't accidentally deploy to the wrong account.
const allowedAccountIds = ["493650548257"];
if (!allowedAccountIds.includes(env.account || "")) {
  throw new Error(
    `Unexpected AWS account ID: ${env.account}. Allowed accounts: ${allowedAccountIds.join(", ")}`,
  );
}

const app = new cdk.App();

new ZapStack(app, "ZapStackProd", {
  env,
  domain: "zap.w01.dev",
});
