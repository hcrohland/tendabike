CREATE TABLE notifications (
    id UUID PRIMARY KEY,
    recipient INT NOT NULL REFERENCES users(id),
    lifecycle TEXT NOT NULL CHECK (lifecycle IN ('transient', 'persistent')),
    kind TEXT NOT NULL CHECK (kind IN ('sub_request', 'sub_response', 'sub_cancelled', 'part_registered', 'part_unregistered')),
    subscription_id INT,
    part_id INT,
    message TEXT,
    state TEXT NOT NULL DEFAULT 'new' CHECK (state IN ('new', 'seen')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((subscription_id IS NOT NULL) != (part_id IS NOT NULL))
);

CREATE INDEX idx_notifications_recipient ON notifications(recipient);
