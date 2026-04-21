import type { Config } from "jest";

const config: Config = {
    verbose: true,
    transform: {
        "^.+\\.tsx?$": "ts-jest",
    },
    roots: ["<rootDir>/tests", "<rootDir>/src"],
    moduleNameMapper: {
        "^(.*)\\.js$": "$1",
    }
};
export default config;
