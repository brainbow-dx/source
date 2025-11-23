import { useState } from "react";

import "./sidebar.css";

export default function Sidebar({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <div id="mySidebar" className={`sidebar ${open ? "open" : ""}`}>
        <button type="button" className="closebtn" onClick={() => setOpen(false)}>
          x
        </button>
        <div className="sidebar-links">
          <button type="button">Empty Home</button>
          <button type="button">Empty About</button>
        </div>
      </div>
      <div className={`${open ? "sidebar-open" : ""}`}>
        <button
          type="button"
          className="openbtn"
          onClick={() => {
            setOpen(!open);
          }}
        >
          Open Sidebar
        </button>
        {children}
      </div>
    </>
  );
}