export default {
  input: "../api/target/openapi.json",
  output: "src/api_client",
  plugins: [
    // ...other plugins
    "@tanstack/react-query",
  ],
};
