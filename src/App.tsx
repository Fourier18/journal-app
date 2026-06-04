import { useEffect } from "react";
import { vaultStatus, listEntries } from "./lib/commands";
import { useVaultStore } from "./store/vault";
import LockScreen from "./components/LockScreen";
import Sidebar from "./components/Sidebar";
import Editor from "./components/Editor";
import NewEntryModal from "./components/NewEntryModal";
import "./styles/tokens.css";
import "./App.css";

export default function App() {
  const { status, showNewEntryModal, setStatus, setEntries } = useVaultStore();

  useEffect(() => {
    vaultStatus().then(setStatus);
  }, []);

  async function handleUnlocked() {
    setStatus("unlocked");
    const entries = await listEntries();
    setEntries(entries);
  }

  if (status === "no_vault") {
    return <LockScreen mode="create" onSuccess={handleUnlocked} />;
  }

  if (status === "locked") {
    return <LockScreen mode="unlock" onSuccess={handleUnlocked} />;
  }

  return (
    <div className="app-layout">
      <Sidebar />
      <Editor />
      {showNewEntryModal && <NewEntryModal />}
    </div>
  );
}
