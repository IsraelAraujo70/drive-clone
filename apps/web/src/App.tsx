import { Route, Routes } from "react-router-dom";
import "./App.css";
import { RequireAuth } from "./lib/auth";
import Drive from "./pages/Drive";
import Landing from "./pages/Landing";
import Login from "./pages/Login";
import Signup from "./pages/Signup";

function App() {
  return (
    <Routes>
      <Route path="/" element={<Landing />} />
      <Route path="/login" element={<Login />} />
      <Route path="/signup" element={<Signup />} />
      <Route
        path="/drive"
        element={
          <RequireAuth>
            <Drive />
          </RequireAuth>
        }
      />
      <Route path="*" element={<Landing />} />
    </Routes>
  );
}

export default App;
