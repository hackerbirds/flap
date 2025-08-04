import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { listen } from '@tauri-apps/api/event'

function App() {
  const [sendTicket, setSendTicket] = useState("");
  const [receiveTicket, setReceiveTicket] = useState("");
  const [filePath, setFilePath] = useState("");
  const [isCrowFlying, setCrowFlying] = useState(false);

  listen('tauri://drag-drop', event => {
    let file_path: string = (event as any).payload.paths[0]
    setFilePath(file_path.split('/')[-1])
    invoke('send_file', { filePath: file_path }).then((t) => {
      setCrowFlying(true)
      setSendTicket(t as string)
    })
  })

  return (
    <main className="container">
      <div className="center">
        <div className="crow">
          {isCrowFlying ? <div className="flying-crow">
            <img className="flying-crow-1" src="flap1.png"></img>
            <img className="flying-crow-2" src="flap2.png"></img>
            <img className="flying-crow-3" src="flap3.png"></img>
          </div>
            : <img className="standing-crow" src="standing.png"></img>}

        </div>
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
                onChange={(e) => { setReceiveTicket(e.target.value) }}
                placeholder="flap/blobadahfshojmu2..."
              />
            </form>
          </section>
        </section>
      </div>
    </main>
  );
}

export default App;
