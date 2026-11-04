import ViewHeader from "../components/ViewHeader";
import { navigate } from "../state/view";

const closeSettings = () => navigate("main");

function SettingsView() {
  return (
    <main class="view">
      <ViewHeader title="Settings" actionLabel="Back" onAction={closeSettings} />
    </main>
  );
}

export default SettingsView;
