import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import { App } from "./App";
import { RpcProvider } from "./rpc/context";
import { selectClient } from "./shell/select";

const client = await selectClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <RpcProvider client={client}>
      <App />
    </RpcProvider>
  </StrictMode>,
);
