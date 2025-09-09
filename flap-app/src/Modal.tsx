import { ReactNode, useRef } from "react";
import "./Modal.css";

interface ModalProps {
    button: ReactNode;
    children: ReactNode;
};

export const Modal: React.FC<ModalProps> = (props) => {
    const modalRef = useRef<HTMLDialogElement>(null);

    const openModal = () => {
        modalRef.current?.showModal()
    }

    const closeModal = () => {
        modalRef.current?.close()
    }

    return <section className="modal-section">
        <button onClick={openModal} className="modal-button">{props.button}</button>
        <dialog ref={modalRef} id="modal" className="modal">
            <button onClick={closeModal} id="closeModal" className="modal-close-button">
                <img className="icon" src="x.svg" />
            </button>
            {props.children}
        </dialog>
    </section>;
}