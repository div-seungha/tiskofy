import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AiOutlineLoading } from "react-icons/ai";
// import { open } from "@tauri-apps/plugin-dialog";
import {
  background,
  input_box,
  input,
  download_button,
  loading,
  description_box,
  table,
  status_style,
  footer,
  th,
  head,
  footer_link,
  loading_container,
} from "./styles.css";

type Status =
  | "completed"
  | "invalid-url"
  | "none"
  | "processing"
  | "canceled"
  | "unknown";

type ErrorMsg = {
  [key: string]: string;
};

const ERROR_MSG: ErrorMsg = {
  completed: "✅ 다운로드가 완료되었습니다.",
  "invalid-url": "❓ 유튜브 영상의 URL을 다시 확인해 주세요.",
  unknown: "❌ 예기치 못한 에러가 발생하였습니다.",
  processing: "📁 지금 열심히 유튜브 클립에서 오디오를 추출하고 있습니다...",
  canceled:
    "🥺 앗, 취소하셨네요! 괜찮아요. 다시 버튼을 누르고 진행하시면 됩니다.",
  none: "...",
};

function App() {
  const [url, setUrl] = useState("");
  const [status, setStatus] = useState<Status>("none");
  const [isLoading, setIsLoading] = useState(false);
  const [err, setErr] = useState("");

  async function handleDownload() {
    if (isLoading) {
      setStatus("canceled");
      setIsLoading(false);
      return;
    }

    if (!url.trim()) {
      setStatus("invalid-url");
      return;
    }

    setIsLoading(true);

    setStatus("processing");
    try {
      const response = await invoke<string>("download_mp3", { url });
      console.log(response);
      if (response.startsWith("Error")) {
        setStatus("unknown");
        setIsLoading(false);
        return;
      }
      if (response.startsWith("Ok")) {
        setStatus("completed");
        setIsLoading(false);
        return;
      }
      if (response.startsWith("canceled")) {
        setStatus("canceled");
        setIsLoading(false);
        return;
      }
      if (response.startsWith("invalid")) {
        setStatus("invalid-url");
        setIsLoading(false);
        return;
      }
      setIsLoading(false);
    } catch (error: any) {
      setIsLoading(false);
      setStatus("unknown");
      setErr(error);
    }
  }

  return (
    <>
      <div className={background}>
        <header>
          <h1 className={head}>Get the mp3 file from the Youtube URL</h1>
        </header>
        <main>
          <div className={input_box}>
            <input
              // disabled
              className={input}
              type="text"
              placeholder="Copy & Paste the Youtube URL..."
              name="url"
              value={url}
              onChange={({ target }) => setUrl(target.value)}
            />
            <button
              // disabled
              className={download_button}
              onClick={handleDownload}
            >
              {isLoading ? (
                <span className={loading}>
                  <AiOutlineLoading />
                </span>
              ) : (
                "Download"
              )}
            </button>
          </div>
          <div className={description_box}>
            <p style={{ fontSize: 12 }}>Ver 1.0.0-beta.3</p>
            <p className={status_style}>{ERROR_MSG[status]}</p>
            <p>{status ? "status: " + status : "--"}</p>
            <div className={loading_container}>
              {isLoading && (
                <span className={loading}>
                  <AiOutlineLoading />
                </span>
              )}
            </div>

            <p>{err || "------"}</p>
            {!isLoading && (
              <span>
                유튜브 URL을 복사 & 붙여넣기하여 유튜브의 오디오를 파일로
                추출해보세요.
              </span>
            )}
            <br />
            <br />
            <p>
              대한민국에서는 음원을 무단으로 사용할 경우 <br />
              저작권법, 정보통신망 이용촉진 및 정보보호 등에 관한 법률 등에
              의하여 처벌받을 수 있습니다.
            </p>
            <table className={table}>
              <thead>
                <tr>
                  <th className={th}>무단 사용 사례</th>
                  <th className={th}>관련 법률 조항</th>
                  <th className={th}>대략적인 처벌 수준</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>음원을 무단 다운로드하여 사용</td>
                  <td>저작권법 제125조</td>
                  <td>민사 손해배상</td>
                </tr>
                <tr>
                  <td>저작권자의 동의 없이 음원을 스트리밍하는 경우</td>
                  <td>저작권법 제136조</td>
                  <td>5년 이하의 징역 또는 5천만 원 이하의 벌금</td>
                </tr>
                <tr>
                  <td>유튜브 등에 음원을 무단으로 업로드하는 경우</td>
                  <td>저작권법 제136조</td>
                  <td>5년 이하의 징역 또는 5천만 원 이하의 벌금</td>
                </tr>
                <tr>
                  <td>불법 사이트에서 음원을 배포하여 수익을 창출하는 경우</td>
                  <td>저작권법 제137조</td>
                  <td>7년 이하의 징역 또는 1억 원 이하의 벌금</td>
                </tr>
              </tbody>
            </table>
          </div>
        </main>
      </div>
      <footer className={footer}>
        &copy; 2025 Designed by{" "}
        <a className={footer_link} href="https://beonanotherplanet.com">
          {" "}
          Seungha Kim
        </a>
        .
      </footer>
    </>
  );
}

export default App;
