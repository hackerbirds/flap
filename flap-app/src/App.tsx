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
  isCompleted: boolean;
};

type PreparingFileEvent = {
  fileTransferId: TransferId;
  metadata: FileMetadata;
  sending: boolean;
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
  const [transfersInProgress, setTransfersInProgress] = useState(0);
  const [transfers, setTransfers] = useState<Map<string, Transfer>>(new Map());

  useEffect(() => {
    invoke('get_send_ticket').then((ticket_string) => setSendTicket(ticket_string as string))

    listen<PreparingFileEvent>('preparing-file', (event) => {
      setTransfersInProgress(transfersInProgress + 1)
      setCrowFlying(true)
      setTransfers(new Map(transfers).set(event.payload.fileTransferId.toString(), {
        sending: event.payload.sending,
        metadata: event.payload.metadata,
        progress: 0,
        isCompleted: false,
      }))
    });

    listen('tauri://drag-drop', event => {
      let file_path: string = (event as any).payload.paths[0]
      setFilePath(file_path.split('/')[-1])
      invoke('send_file', { filePath: file_path });
    })

    listen<TransferCompleteEvent>('transfer-complete', (event) => {
      console.log("yeah")
      const newTransfers = new Map(transfers)
      newTransfers.delete(event.payload.fileTransferId.toString())
      setTransfers(newTransfers)
      // this was the last transfer
      if (transfersInProgress === 1) {
        setCrowFlying(false)
      }
      setTransfersInProgress(transfersInProgress - 1)
    })
  }, []);

  useEffect(() => {
    listen<TransferUpdateEvent>('transfer-update', (event) => {
      let transfer = transfers.get(event.payload.fileTransferId.toString())
      if (transfer) {
        setTransfers(new Map(transfers).set(event.payload.fileTransferId.toString(), {
          sending: transfer.sending,
          metadata: transfer.metadata,
          progress: (100 * event.payload.bytesDownloaded) / transfer.metadata.expectedFileSize,
          isCompleted: false,
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
            <div className="transfers sending">
              {
                [...transfers].filter(([_transfer_id, transfer]) => transfer.sending).map(([transfer_id, transfer]) => {
                  return <div className="transfer" key={transfer_id}>
                    <b>{transfer.metadata.fileName}</b>
                    <progress max="100" value={transfer.progress === 0 ? undefined : transfer.progress}></progress>
                  </div>
                })
              }
            </div>
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
            <div className="transfers receiving">
              {
                [...transfers].filter(([_transfer_id, transfer]) => !transfer.sending).map(([transfer_id, transfer]) => {
                  return <div className="transfer" key={transfer_id}>
                    <b>{transfer.metadata.fileName}</b>
                    <progress max="100" value={transfer.progress}></progress>
                  </div>
                })
              }
            </div>
          </section>
        </section>
      </div>
    </main>
  );
}

export default App;
