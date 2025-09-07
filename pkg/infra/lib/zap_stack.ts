import * as cdk from "aws-cdk-lib";
import * as s3 from "aws-cdk-lib/aws-s3";
import * as s3deploy from "aws-cdk-lib/aws-s3-deployment";
import * as cloudfront from "aws-cdk-lib/aws-cloudfront";
import * as origins from "aws-cdk-lib/aws-cloudfront-origins";
import * as acm from "aws-cdk-lib/aws-certificatemanager";
import * as route53 from "aws-cdk-lib/aws-route53";
import * as targets from "aws-cdk-lib/aws-route53-targets";
import * as lambda from "aws-cdk-lib/aws-lambda";
import * as secretsmanager from "aws-cdk-lib/aws-secretsmanager";
import * as iam from "aws-cdk-lib/aws-iam";
import * as rds from "aws-cdk-lib/aws-rds";
import * as ec2 from "aws-cdk-lib/aws-ec2";
import { RemovalPolicy, Duration } from "aws-cdk-lib";
import { Construct } from "constructs";
import * as fs from "fs";

export interface ZapStackProps extends cdk.StackProps {
  domain: string;
}

export class ZapStack extends cdk.Stack {
  public readonly webBucket: s3.Bucket;
  public readonly distribution: cloudfront.Distribution;
  public readonly apiSecret: secretsmanager.Secret;
  public readonly dbCluster: rds.DatabaseCluster;
  public readonly apiFunction: lambda.Function;

  constructor(scope: Construct, id: string, props: ZapStackProps) {
    super(scope, id, props);
    const webDistDir = __dirname + "/../../web/dist";

    const vpc = ec2.Vpc.fromLookup(this, "DefaultVpc", { isDefault: true });

    // Create SecretsManager secret for API keys
    this.apiSecret = new secretsmanager.Secret(this, "ApiKeys", {
      secretName: "api_keys",
      description: "Environment variables for the API Lambda function",
      generateSecretString: {
        secretStringTemplate: JSON.stringify({}),
        generateStringKey: "placeholder",
      },
    });

    // Create Aurora Serverless v2 cluster
    this.dbCluster = new rds.DatabaseCluster(this, "DbCluster", {
      defaultDatabaseName: "app",
      engine: rds.DatabaseClusterEngine.auroraPostgres({
        version: rds.AuroraPostgresEngineVersion.VER_17_4,
      }),

      // Provision a single writer instance.
      writer: rds.ClusterInstance.serverlessV2("ClusterInstance"),

      // Put it in the public subnet (...not great, but I'm cheap)
      vpc,
      vpcSubnets: { subnetType: ec2.SubnetType.PUBLIC },

      // Store credentials in a SecretsManager secret
      credentials: rds.Credentials.fromGeneratedSecret("postgres"),

      // Enable Aurora Serverless v2
      serverlessV2MinCapacity: 0,
      serverlessV2MaxCapacity: 1,
      serverlessV2AutoPauseDuration: Duration.minutes(5),

      // Encrypt storage.
      storageEncrypted: true,

      // Don't remove data.
      deletionProtection: true,
      removalPolicy: RemovalPolicy.SNAPSHOT,

      enableDataApi: true,
    });

    // Allow inbound connections on PostgreSQL port from anywhere (public access)
    this.dbCluster.connections.allowDefaultPortFromAnyIpv4(
      "Allow PostgreSQL connections from anywhere",
    );

    // Create Lambda function for API
    this.apiFunction = new lambda.Function(this, "ApiFunction", {
      runtime: lambda.Runtime.PROVIDED_AL2023,
      architecture: lambda.Architecture.ARM_64,
      handler: "bootstrap",
      code: lambda.Code.fromAsset("../api/target/lambda/lambda_server"),
      timeout: cdk.Duration.seconds(30),
      memorySize: 512,
      environment: {
        ROCKET_ENV: "production",
        API_SECRET_ARN: this.apiSecret.secretArn,
        DB_SECRET_ARN: this.dbCluster.secret!.secretArn,
      },
    });

    // Grant Lambda permission to read secrets
    this.apiSecret.grantRead(this.apiFunction);
    this.dbCluster.secret!.grantRead(this.apiFunction);

    // Create Function URL with no authentication
    const functionUrl = this.apiFunction.addFunctionUrl({
      authType: lambda.FunctionUrlAuthType.NONE,
      cors: {
        allowedOrigins: [`https://${props.domain}`],
        allowedMethods: [lambda.HttpMethod.ALL],
        allowedHeaders: ["*"],
      },
    });

    // S3 bucket for webapp resources.
    this.webBucket = new s3.Bucket(this, "WebBucket", {
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    // Look up the hosted zone
    const hostedZone = route53.HostedZone.fromLookup(this, "HostedZone", {
      domainName: props.domain,
    });

    // Create SSL certificate for the domain in us-east-1 (required for CloudFront)
    const certificate = new acm.DnsValidatedCertificate(this, "Certificate", {
      domainName: props.domain,
      subjectAlternativeNames: [`www.${props.domain}`],
      hostedZone,
      region: "us-east-1",
    });

    // Create CloudFront function for SPA routing
    const spaRoutingFunction = new cloudfront.Function(
      this,
      "SpaRoutingFunction",
      {
        code: cloudfront.FunctionCode.fromInline(
          generateSpaRoutingFunctionCode(webDistDir),
        ),

        // Use the 2.0 runtime
        runtime: cloudfront.FunctionRuntime.JS_2_0,
      },
    );

    // Create CloudFront distribution
    this.distribution = new cloudfront.Distribution(this, "Distribution", {
      defaultBehavior: {
        origin: new origins.S3Origin(this.webBucket),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        functionAssociations: [
          {
            function: spaRoutingFunction,
            eventType: cloudfront.FunctionEventType.VIEWER_REQUEST,
          },
        ],
      },
      additionalBehaviors: {
        "/api/*": {
          origin: new origins.FunctionUrlOrigin(functionUrl),
          viewerProtocolPolicy:
            cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
          cachePolicy: cloudfront.CachePolicy.CACHING_DISABLED,
          originRequestPolicy:
            cloudfront.OriginRequestPolicy.ALL_VIEWER_EXCEPT_HOST_HEADER,
          allowedMethods: cloudfront.AllowedMethods.ALLOW_ALL,
        },
      },
      domainNames: [props.domain, `www.${props.domain}`],
      certificate,
      defaultRootObject: "index.html",
    });

    // Create Route53 A records
    new route53.ARecord(this, "ApexRecord", {
      zone: hostedZone,
      recordName: props.domain,
      target: route53.RecordTarget.fromAlias(
        new targets.CloudFrontTarget(this.distribution),
      ),
    });

    new route53.ARecord(this, "WwwRecord", {
      zone: hostedZone,
      recordName: `www.${props.domain}`,
      target: route53.RecordTarget.fromAlias(
        new targets.CloudFrontTarget(this.distribution),
      ),
    });

    // Create a BucketDeployment to deploy static files to the S3 bucket.
    new s3deploy.BucketDeployment(this, "WebBucketDeployment", {
      sources: [s3deploy.Source.asset(webDistDir)],
      destinationBucket: this.webBucket,
      distribution: this.distribution,
      distributionPaths: ["/*"],
    });

    // Stack outputs
    new cdk.CfnOutput(this, "ApiSecretArn", {
      value: this.apiSecret.secretArn,
      description: "ARN of the API keys secret",
    });

    new cdk.CfnOutput(this, "DbSecretArn", {
      value: this.dbCluster.secret!.secretArn,
      description: "ARN of the database credentials secret",
    });
  }
}

function generateSpaRoutingFunctionCode(webappDistDir: string): string {
  // We want to generate a list of URL prefixes which should ALWAYS be
  // routed to S3, and NOT subject to the SPA index.html routing rules.
  // 1. We'll list the webappDistDir
  // 2. For files, we'll append the filaname.
  // 3. For directories, we'll append the directory name with a trailing slash.
  //
  // Then, in the function, we'll prefix-check the request URI against
  // this list, and if it matches any of them, we'll skip the index.html
  // routing logic.

  const alwaysS3Prefixes: string[] = [];
  const items = fs.readdirSync(webappDistDir, { withFileTypes: true });
  for (const item of items) {
    if (item.isFile()) {
      alwaysS3Prefixes.push(`/${item.name}`);
    } else if (item.isDirectory()) {
      alwaysS3Prefixes.push(`/${item.name}/`);
    }
  }

  return `
    const alwaysS3Prefixes = ${JSON.stringify(alwaysS3Prefixes)};

    function handler(event) {
      const request = event.request;
      const uri = request.uri;
      
      // Don't redirect API routes
      if (uri.startsWith('/api/')) {
        console.log("api route: " + JSON.stringify({uri, prefix}));
        return request;
      }

      // Don't redirect if the URI matches any of the always-S3 prefixes
      for (let i = 0; i < alwaysS3Prefixes.length; i++) {
        const prefix = alwaysS3Prefixes[i];
        if (uri.startsWith(prefix)) {
          console.log("asset match: ", JSON.stringify({uri, prefix}));
          return request;
        }
      }

      // Otherwise, reroute to 'index.html'
      console.log("spa redirect: " + JSON.stringify({uri}));
      request.uri = '/index.html';
      return request;
    }
  `;
}
