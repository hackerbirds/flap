import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { listen } from '@tauri-apps/api/event'

type TransferId = Uint8Array;

type Transfer = {
  // `true` if sending, `false` is receiving
  sending: boolean;
  metadata: FileMetadata;
  progress: number;
};

type ReceivingFileEvent = {
  fileTransferId: TransferId;
  metadata: FileMetadata;
};

type FileMetadata = {
  fileName: string;
  expectedFileSize: number;
};

type TransferUpdateEvent = {
  fileTransferId: TransferId;
  bytesDownloaded: number;
};

type TransferCompleteEvent = {
  fileTransferId: TransferId;
};

function App() {
  const [sendTicket, setSendTicket] = useState("");
  const [receiveTicket, setReceiveTicket] = useState("");
  const [filePath, setFilePath] = useState("");
  const [isCrowFlying, setCrowFlying] = useState(false);
  const [transfers, setTransfers] = useState<Map<string, Transfer>>(new Map());

  useEffect(() => {
    invoke('get_send_ticket').then((ticket_string) => setSendTicket(ticket_string as string))

    listen<ReceivingFileEvent>('receiving-file', (event) => {
      console.log("here " + event.payload.fileTransferId)
      setCrowFlying(true)
      setTransfers(new Map(transfers).set(event.payload.fileTransferId.toString(), {
        sending: false,
        metadata: event.payload.metadata,
        progress: 0,
      }))
    });

    listen('tauri://drag-drop', event => {
      let file_path: string = (event as any).payload.paths[0]
      setFilePath(file_path.split('/')[-1])
      invoke('send_file', { filePath: file_path }).then(() => {
        setCrowFlying(true)
      })
    })

    listen<TransferCompleteEvent>('transfer-complete', (event) => {
      const newTransfers = new Map(transfers)
      newTransfers.delete(event.payload.fileTransferId.toString())
      if (newTransfers.size === 0) {
        setCrowFlying(false)
      }
      setTransfers(newTransfers)
    })
  }, []);

  useEffect(() => {
    listen<TransferUpdateEvent>('transfer-update', (event) => {
      let transfer = transfers.get(event.payload.fileTransferId.toString())
      if (transfer) {
        setTransfers(new Map(transfers).set(event.payload.fileTransferId.toString(), {
          sending: transfer.sending,
          metadata: transfer.metadata,
          progress: (100 * event.payload.bytesDownloaded) / transfer.metadata.expectedFileSize
        }))
      }
    })
  }, [transfers, setTransfers]);

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
                  console.log("File received")
                })
              }}
            >
              <input
                id="ticket-input"
                onChange={(e) => { setReceiveTicket(e.target.value) }}
                placeholder="flap/<id>/<key>"
              />
            </form>
            <>
              {
                [...transfers].map(([transfer_id, transfer]) => {
                  return <div className="transfer" key={transfer_id}>
                    <b>{transfer.metadata.fileName}</b>
                    <progress max="100" value={transfer.progress}></progress>
                  </div>
                })
              }
            </>
          </section>
        </section>
      </div>
    </main>
  );
}

export default App;
