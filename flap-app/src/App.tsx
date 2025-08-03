import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { listen } from '@tauri-apps/api/event'

function App() {
  const [sendTicket, setSendTicket] = useState("");
  const [receiveTicket, setReceiveTicket] = useState("");
  const [filePath, setFilePath] = useState("");

  listen('tauri://drag-drop', event => {
    let file_path: string = (event as any).payload.paths[0]
    setFilePath(file_path.split('/')[-1])
    invoke('send_file', { filePath: file_path }).then((t) => {
      setSendTicket(t as string)
    })
  })

  return (
    <main className="container">
      <div className="center">
        <h1>Flap</h1>
        <section id="action">
          <section id="send">
            <h1>Send a package</h1>
            <i>{filePath}</i>
            <span id="ticket">{sendTicket}</span>
          </section>
          <section id="receive">
            <h1>Receive a package</h1>
            <form
              className="row"
              onSubmit={(e) => {
                e.preventDefault();
                invoke('receive_file', { ticketString: receiveTicket }).then(() => {
                  console.log("yay")
                })
              }}
            >
              <input
                id="ticket-input"
                onChange={(e) => {setReceiveTicket(e.target.value)}}
                placeholder="Enter ticket"
              />
            </form>
          </section>
        </section>
      </div>
    </main>
  );
}

export default App;
