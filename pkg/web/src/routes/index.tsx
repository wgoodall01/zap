import { redirect, createFileRoute } from "@tanstack/react-router";
import logo from "../logo.svg";
import * as tg from "@/telegram";

export const Route = createFileRoute("/")({
  component: App,
  beforeLoad: () => {
    throw redirect({ to: "/zap" });
  },
});

function App() {
  const rawInitData = tg.getRawInitData();

  return (
    <div className="App">
      <header className="App-header">
        <img src={logo} className="App-logo" alt="logo" />
        <p>
          Edit <code>src/routes/index.tsx</code> and save to reload.
        </p>
        <p>{JSON.stringify(rawInitData)}</p>
        <a
          className="App-link"
          href="https://reactjs.org"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn React
        </a>
        <a
          className="App-link"
          href="https://tanstack.com"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn TanStack
        </a>
      </header>
    </div>
  );
}
