import { type FormEvent, useEffect, useState } from "react";
import { getSoul, updateSoul } from "./soul";

type SoulEditorProps = {
  onError: (message?: string) => void;
};

export function SoulEditor({ onError }: SoulEditorProps) {
  const [content, setContent] = useState("");
  const [saved, setSaved] = useState(false);
  const [version, setVersion] = useState(1);

  useEffect(() => {
    void getSoul()
      .then((document) => {
        setContent(document.content);
        setVersion(document.soulVersion);
      })
      .catch((error: unknown) => onError(String(error)));
  }, [onError]);

  async function save(event: FormEvent) {
    event.preventDefault();
    onError(undefined);
    setSaved(false);
    try {
      const document = await updateSoul(content);
      setContent(document.content);
      setVersion(document.soulVersion);
      setSaved(true);
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <form className="soul-editor" onSubmit={(event) => void save(event)}>
      <label>
        SOUL.md（身份 source of truth，v{version}）
        <textarea
          value={content}
          onChange={(event) => setContent(event.target.value)}
          required
        />
      </label>
      <button type="submit">{saved ? "Soul 已保存" : "保存 Soul"}</button>
    </form>
  );
}
