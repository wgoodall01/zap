#!/usr/bin/env node
import * as cdk from "aws-cdk-lib";
import { ZapStack } from "../lib/zap_stack";

// AWS account and region.
const env = {
  account: process.env.CDK_DEFAULT_ACCOUNT,
  region: process.env.CDK_DEFAULT_REGION,
};

const app = new cdk.App();

new ZapStack(app, "ZapStackProd", {
  env,
  domain: "zap.w01.dev",
});
