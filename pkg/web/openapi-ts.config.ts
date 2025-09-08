export default {
  input: "../api/target/openapi.json",
  output: "src/api_client",
  plugins: [
    // ...other plugins
    {
      name: "@tanstack/react-query",

      // Don't prefix/suffix names. OperationIds are already globally unique.
      mutationOptions: { name: `mutation{{name}}` },
      queryOptions: { name: `query{{name}}` },
    },
  ],
};
