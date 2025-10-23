#!/usr/bin/env node
import * as cdk from "aws-cdk-lib";
import { ZapStack } from "../lib/zap_stack";
import { GithubOidcStack } from "../lib/github-oidc-stack";
import * as fs from "fs";
import * as path from "path";
import { z } from "zod";

// Load account configuration
const accountConfigPath = path.join(__dirname, "../account_ids.json");
const accountConfig = z
  .object({
    prod: z.object({
      accountId: z.string(),
      region: z.string(),
    }),
  })
  .parse(JSON.parse(fs.readFileSync(accountConfigPath, "utf-8")));

// AWS account and region from config
const env = {
  account: accountConfig.prod.accountId,
  region: accountConfig.prod.region,
};

const app = new cdk.App();

new ZapStack(app, "ZapStackProd", {
  env,
  domain: "zap.w01.dev",
});

// GitHub Actions OIDC integration for CI/CD
new GithubOidcStack(app, "GithubOidcStack", {
  env,
  githubRepository: "wgoodall01/zap",
  allowedBranch: "main",
});
