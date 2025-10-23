import * as cdk from "aws-cdk-lib";
import * as iam from "aws-cdk-lib/aws-iam";
import { Construct } from "constructs";

export interface GithubOidcStackProps extends cdk.StackProps {
  /**
   * GitHub repository in format "owner/repo"
   * Example: "wgoodall01/zap"
   */
  githubRepository: string;

  /**
   * Branch that is allowed to assume the role
   * Example: "main"
   */
  allowedBranch: string;
}

export class GithubOidcStack extends cdk.Stack {
  public readonly deployRole: iam.Role;
  public readonly oidcProvider: iam.OpenIdConnectProvider;

  constructor(scope: Construct, id: string, props: GithubOidcStackProps) {
    super(scope, id, props);

    // Create OIDC provider for GitHub Actions
    // As of 2025, AWS trusts GitHub's CA, so no thumbprint is required
    this.oidcProvider = new iam.OpenIdConnectProvider(
      this,
      "GithubOidcProvider",
      {
        url: "https://token.actions.githubusercontent.com",
        clientIds: ["sts.amazonaws.com"],
      },
    );

    // Create IAM role that GitHub Actions can assume
    this.deployRole = new iam.Role(this, "GithubActionsDeployRole", {
      roleName: "GithubActionsDeployRole",
      description:
        "Role assumed by GitHub Actions for deploying via CDK from the main branch",
      maxSessionDuration: cdk.Duration.hours(4),

      // Configure trust policy to only allow specific repository and branch
      assumedBy: new iam.WebIdentityPrincipal(
        this.oidcProvider.openIdConnectProviderArn,
        {
          StringEquals: {
            // Verify the token is for AWS STS
            "token.actions.githubusercontent.com:aud": "sts.amazonaws.com",
            // Only allow the specified repository and branch
            "token.actions.githubusercontent.com:sub": `repo:${props.githubRepository}:ref:refs/heads/${props.allowedBranch}`,
          },
        },
      ),

      // Grant AdministratorAccess - full AWS access needed for deploying IaC with IAM resources
      managedPolicies: [
        iam.ManagedPolicy.fromAwsManagedPolicyName("AdministratorAccess"),
      ],
    });

    // Output the role ARN for use in GitHub Actions workflows
    new cdk.CfnOutput(this, "DeployRoleArn", {
      value: this.deployRole.roleArn,
      description:
        "ARN of the IAM role for GitHub Actions to assume (use in aws-actions/configure-aws-credentials)",
      exportName: "GithubActionsDeployRoleArn",
    });

    // Output helpful information
    new cdk.CfnOutput(this, "AllowedRepository", {
      value: props.githubRepository,
      description: "GitHub repository allowed to assume this role",
    });

    new cdk.CfnOutput(this, "AllowedBranch", {
      value: props.allowedBranch,
      description: "Git branch allowed to assume this role",
    });
  }
}
