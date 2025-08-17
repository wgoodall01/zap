import * as cdk from "aws-cdk-lib";
import * as s3 from "aws-cdk-lib/aws-s3";
import * as s3deploy from "aws-cdk-lib/aws-s3-deployment";
import * as cloudfront from "aws-cdk-lib/aws-cloudfront";
import * as origins from "aws-cdk-lib/aws-cloudfront-origins";
import * as acm from "aws-cdk-lib/aws-certificatemanager";
import * as route53 from "aws-cdk-lib/aws-route53";
import * as targets from "aws-cdk-lib/aws-route53-targets";
import { Construct } from "constructs";

export interface ZapStackProps extends cdk.StackProps {
  domain: string;
}

export class ZapStack extends cdk.Stack {
  public readonly webBucket: s3.Bucket;
  public readonly distribution: cloudfront.Distribution;

  constructor(scope: Construct, id: string, props: ZapStackProps) {
    super(scope, id, props);

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

    // Create CloudFront distribution
    this.distribution = new cloudfront.Distribution(this, "Distribution", {
      defaultBehavior: {
        origin: new origins.S3Origin(this.webBucket),
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
      },
      domainNames: [props.domain, `www.${props.domain}`],
      certificate,
      defaultRootObject: "index.html",
      errorResponses: [
        {
          httpStatus: 404,
          responseHttpStatus: 200,
          responsePagePath: "/index.html",
        },
      ],
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
      sources: [s3deploy.Source.asset(__dirname + "/../../web/dist")],
      destinationBucket: this.webBucket,
      distribution: this.distribution,
      distributionPaths: ["/*"],
    });
  }
}
