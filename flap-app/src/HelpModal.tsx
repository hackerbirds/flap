import { Modal } from "./Modal";

export default function HelpModal() {
    return <Modal
        button={<img className="icon" src="help-circle.svg" />}
    >
        <h3>HOW DO I USE THIS?</h3>
    </Modal>;
}