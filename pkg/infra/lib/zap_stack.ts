import * as cdk from "aws-cdk-lib";
import * as s3 from "aws-cdk-lib/aws-s3";
import * as s3deploy from "aws-cdk-lib/aws-s3-deployment";
import { Construct } from "constructs";

export class ZapStack extends cdk.Stack {
  public readonly webBucket: s3.Bucket;

  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // S3 bucket for webapp resources.
    this.webBucket = new s3.Bucket(this, "WebBucket", {
      removalPolicy: cdk.RemovalPolicy.DESTROY,
    });

    // Create a BucketDeployment to deploy static files to the S3 bucket.
    new s3deploy.BucketDeployment(this, "WebBucketDeployment", {
      sources: [s3deploy.Source.asset(__dirname + "/../../web/dist")],
      destinationBucket: this.webBucket,
    });

    // The code that defines your stack goes here
  }
}
