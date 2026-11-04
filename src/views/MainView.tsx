import ViewHeader from "../components/ViewHeader";
import { navigate } from "../state/view";

const openSettings = () => navigate("settings");

function MainView() {
  return (
    <main class="view">
      <ViewHeader title="voice-to-tts" actionLabel="Settings" onAction={openSettings} />
    </main>
  );
}

export default MainView;
