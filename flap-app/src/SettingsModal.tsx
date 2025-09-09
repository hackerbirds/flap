import { Modal } from "./Modal";

export default function SettingsModal() {
    return <Modal
        button={<img className="icon" src="settings.svg" />}
    >
        <b>Settings modal</b>
    </Modal>;
}