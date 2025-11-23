-- Enable foreign key constraints. This must be set for each database connection.
PRAGMA foreign_keys = ON;

-- Table: user
CREATE TABLE user (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    email TEXT UNIQUE,
    password_hash TEXT,
    display_name TEXT,
    pic_url TEXT,
    default_loc TEXT,
    latitude REAL,
    longitude REAL,
    joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_login DATETIME
);

CREATE INDEX idx_user_username ON user (username);
CREATE INDEX idx_user_email ON user (email);

-- Table: category
CREATE TABLE category (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT
);

INSERT INTO category (name)
VALUES
    ('Furniture'),
    ('Electronics'),
    ('Books'),
    ('Apparel'),
    ('Home Decor'),
    ('Kitchenware'),
    ('Outdoor & Garden'),
    ('Sports & Fitness'),
    ('Tools'),
    ('Collectibles'),
    ('Services');

-- Table: sku
CREATE TABLE sku (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sku_code TEXT NOT NULL UNIQUE,
    category_id INTEGER NOT NULL,
    
    initial_description TEXT,
    condition TEXT,
    is_service BOOLEAN NOT NULL DEFAULT 0,

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (category_id) REFERENCES category(id) ON DELETE RESTRICT ON UPDATE CASCADE
);

CREATE INDEX idx_sku_sku_code ON sku (sku_code);
CREATE INDEX idx_sku_category_id ON sku (category_id);

-- Table: kind
CREATE TABLE kind (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT
);

INSERT INTO kind (name, description)
VALUES
    ('notice', 'A general notice or offering for an item or service.'),
    ('treasure', 'A particularly desirable or unique item, often a special find.'),
    ('service', 'An offering for a service rather than a physical item.');

-- Table: listing
CREATE TABLE listing (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sku_id INTEGER NOT NULL,
    kind_id INTEGER NOT NULL,
    poster_id INTEGER NOT NULL,
    
    title TEXT NOT NULL,
    description TEXT,
    
    current_loc TEXT,
    lat REAL,
    lon REAL,
    
    status TEXT NOT NULL DEFAULT 'available',
    claimer_id INTEGER,
    
    posted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME,
    
    FOREIGN KEY (sku_id) REFERENCES sku(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    FOREIGN KEY (kind_id) REFERENCES kind(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    FOREIGN KEY (poster_id) REFERENCES user(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    FOREIGN KEY (claimer_id) REFERENCES user(id) ON DELETE SET NULL ON UPDATE CASCADE
);

CREATE INDEX idx_listing_sku_id ON listing (sku_id);
CREATE INDEX idx_listing_kind_id ON listing (kind_id);
CREATE INDEX idx_listing_poster_id ON listing (poster_id);
CREATE INDEX idx_listing_status ON listing (status);

-- Table: media
CREATE TABLE media (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    listing_id INTEGER NOT NULL,
    
    url TEXT NOT NULL,
    kind TEXT NOT NULL, -- e.g., 'image', 'video', 'audio', 'document'
    alt_text TEXT,
    display_order INTEGER,
    metadata TEXT,      -- Stores JSON string for key-value pairs (e.g., {"size_kb": 500, "duration_sec": 30})

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (listing_id) REFERENCES listing(id) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX idx_media_listing_id ON media (listing_id);
CREATE INDEX idx_media_uuid ON media (uuid);
