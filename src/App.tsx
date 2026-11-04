import { currentView } from "./state/view";
import MainView from "./views/MainView";
import SettingsView from "./views/SettingsView";

function App() {
  return currentView.value === "settings" ? <SettingsView /> : <MainView />;
}

export default App;
