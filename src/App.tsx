import { modelsState } from "./state/models";
import { currentView } from "./state/view";
import GuideView from "./views/GuideView";
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

  switch (currentView.value) {
    case "settings":
      return <SettingsView />;
    case "guide":
      return <GuideView />;
    default:
      return <MainView />;
  }
}

export default App;
