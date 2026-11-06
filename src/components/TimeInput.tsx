import { useEffect, useState } from 'react';
import { parseTime, time } from '../lib';

export function TimeInput({
  value,
  onChange,
  label,
}: {
  value: number;
  onChange: (n: number) => void;
  label: string;
}) {
  const [text, setText] = useState(time(value)),
    [invalid, setInvalid] = useState(false);
  useEffect(() => {
    setText(time(value));
    setInvalid(false);
  }, [value]);
  return (
    <input
      className={'time-input ' + (invalid ? 'invalid' : '')}
      aria-label={label}
      aria-invalid={invalid}
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={() => {
        try {
          onChange(parseTime(text));
          setInvalid(false);
        } catch {
          setInvalid(true);
        }
      }}
      title={invalid ? 'Use M:SS or H:MM:SS' : label}
    />
  );
}
