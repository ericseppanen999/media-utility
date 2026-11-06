import { Download, FileAudio, Info } from 'lucide-react';
import { time } from '../lib';
import type { Plan } from '../types';
import { Modal } from './Modal';

interface PlanReviewModalProps {
  plans: Plan[];
  setPlans: (plans: Plan[]) => void;
  enqueue: () => Promise<void>;
  busy: string;
  error: string;
}

export function PlanReviewModal({ plans, setPlans, enqueue, busy, error }: PlanReviewModalProps) {
  return (
    <Modal
      error={error}
      title="Review export"
      subtitle={plans.reduce((n, p) => n + p.outputs.length, 0) + ' output files'}
      close={() => setPlans([])}
      wide
    >
      <div className="plan-list">
        {plans.map((plan) => (
          <div className="plan-group" key={plan.id}>
            <h3>{plan.source.title}</h3>
            <div className="plan-policy">
              <span className={'pill ' + (plan.copy ? 'green' : '')}>
                {plan.copy ? 'STREAM COPY' : 'RE-ENCODE'}
              </span>
              <p>{plan.explanation}</p>
            </div>
            {plan.warnings.map((w) => (
              <p className="hint warning" key={w}>
                <Info size={14} /> {w}
              </p>
            ))}
            <div className="planned-files">
              {plan.outputs.map((o) => (
                <div key={o.segment.id}>
                  <FileAudio size={15} />
                  <span title={o.path}>{o.path}</span>
                  <small>{time(o.segment.end_us - o.segment.start_us)}</small>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
      <div className="modal-actions">
        <button className="button secondary" onClick={() => setPlans([])}>
          Keep editing
        </button>
        <button className="button primary" disabled={!!busy} onClick={() => void enqueue()}>
          <Download size={16} /> Start export
        </button>
      </div>
    </Modal>
  );
}
