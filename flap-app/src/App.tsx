import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import HelpModal from "./HelpModal";

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
  const [isCrowFlying, setCrowFlying] = useState(false);
  const [transfersInProgress, setTransfersInProgress] = useState(0);

  // A hacky way to map file name -> file path
  const [pendingSendingTransfers, setPendingSendingTransfers] = useState<Map<string, string>>(new Map());
  // The string here can be two things:
  // - A file path (if the user is sending a file)
  // - A transfer id (only known after initiating a transfer with a peer)
  //   - The receiving transfers can only rely on the transfer id, but sending
  //   transfers need a way to be stored before initiating a connection (ie before
  //   obtaining a transfer id)
  const [transfers, setTransfers] = useState<Map<string, Transfer>>(new Map());
  const [completedTransfers, setCompletedTransfers] = useState<FileMetadata[]>([]);

  const copyTicketToClipboard = async () => {
    console.log("Copied ticket to clipboard")
    await writeText(sendTicket);
  }
  const addFile = async (filePath: string) => {
    invoke('send_file', { filePath }).then(() => {
      // TODO: Only for POSIX? Windows compatible? Is there a JS native way of doing this?
      const pathSplit = filePath.split('/')
      const fileName = pathSplit[pathSplit.length - 1]

      setPendingSendingTransfers(new Map(pendingSendingTransfers).set(fileName, filePath))
      setTransfers(new Map(transfers).set(filePath, {
        sending: true,
        metadata: {
          fileName: fileName,
          expectedFileSize: 0
        },
        progress: 0,
        isCompleted: false,
      }))
    });
  }

  useEffect(() => {
    invoke('get_send_ticket').then((ticket_string) => setSendTicket(ticket_string as string))

    listen<PreparingFileEvent>('preparing-file', (event) => {
      setTransfersInProgress(transfersInProgress + 1)
      if (transfersInProgress > 0 && !isCrowFlying)
        setCrowFlying(true)

      const newMap = new Map(transfers)

      if (event.payload.sending) {
        const fileName = event.payload.metadata.fileName;
        const fullFilePath = pendingSendingTransfers.get(fileName)
        if (fullFilePath)
          newMap.delete(fullFilePath)
      }

      newMap.set(event.payload.fileTransferId.toString(), {
        sending: event.payload.sending,
        metadata: event.payload.metadata,
        progress: 0,
        isCompleted: false,
      })

      setTransfers(newMap)
    });

    listen('tauri://drag-drop', event => {
      let filePath: string = (event as any).payload.paths[0]
      addFile(filePath)
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

    listen<TransferCompleteEvent>('transfer-complete', (event) => {
      let transfer = transfers.get(event.payload.fileTransferId.toString())

      console.log("Transfer completed event received")

      if (transfer) {
        setCompletedTransfers([...completedTransfers, transfer.metadata])
        const newTransfers = new Map(transfers)
        newTransfers.delete(event.payload.fileTransferId.toString())
        setTransfers(newTransfers)
      }

      // this was the last transfer
      if (transfersInProgress <= 1) {
        setCrowFlying(false)
      }
      setTransfersInProgress(transfersInProgress - 1)
    })
  }, [transfers, setTransfers]);

  const selectFileDialog = async () => {
    const filePath = await open({
      multiple: false,
      directory: false,
    });

    if (filePath)
      addFile(filePath)
  }

  return (
    <main className="container">
      <div className="center">
        <div className="top">
          <div className="crow">
            {isCrowFlying ? <div className="flying-crow">
              <img className="flying-crow-1" src="flap1.png"></img>
              <img className="flying-crow-2" src="flap2.png"></img>
              <img className="flying-crow-3" src="flap3.png"></img>
            </div>
              : <img className="standing-crow" src="standing.png"></img>}
          </div>
          <section id="top-buttons">
          </section>
        </div>
        <section id="action">
          <section id="send">
            <h1>Send a package</h1>
            <div className="ticket">
              <img className="icon" src="file-plus.svg" onClick={selectFileDialog} alt="Add a new file for this transfer" />
              <h3 onClick={copyTicketToClipboard}>{sendTicket}</h3>
            </div>
            <div className="transfers completed">
              {
                [...completedTransfers].filter((metadata) => pendingSendingTransfers.get(metadata.fileName) !== undefined).map((metadata, i) => {
                  return <div className="completed-transfer transfer" key={i}>
                    <b>{metadata.fileName}</b>
                    <img className="icon" src="check.svg" />
                  </div>
                })
              }
            </div>
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
            <div className="transfers completed">
              {
                [...completedTransfers].filter((metadata) => pendingSendingTransfers.get(metadata.fileName) === undefined).map((metadata, i) => {
                  return <div className="completed-transfer" key={i}>
                    <b>{metadata.fileName}</b>
                    <img src="check.svg" />
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
