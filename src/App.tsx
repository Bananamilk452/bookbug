import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

function App() {
  const [input, setInput] = useState("");

  return (
    <div className="app">
      <input type="text" value={input} onChange={(e) => setInput(e.target.value)} />
      <button
        onClick={() =>
          invoke("test", { input: input })
            .then((response) => {
              alert(response);
            })
            .catch((error) => {
              alert(JSON.stringify(error, null, 2));
              console.error("Error invoking test command:", error);
            })
        }
      >
        Test
      </button>
    </div>
  );
}

export default App;
