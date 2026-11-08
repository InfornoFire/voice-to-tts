import { modelsState } from "./state/models";
import { currentView } from "./state/view";
import MainView from "./views/MainView";
import ModelsView from "./views/ModelsView";
import SettingsView from "./views/SettingsView";

function App() {
  const models = modelsState.value;
  if (models === null) {
    return null;
  }
  if (models.status !== "ready") {
    return <ModelsView state={models} />;
  }
  return currentView.value === "settings" ? <SettingsView /> : <MainView />;
}

export default App;
