const Path = require('path');
const CopyWebpackPlugin = require('copy-webpack-plugin');

module.exports = {
  entry: Path.resolve(__dirname, './src/index.ts'),
  devtool: 'inline-source-map',
  mode: 'production',
  output: {
    path: Path.resolve(__dirname, 'dist'),
    filename: '[name].min.js',
  },
  plugins: [
    new CopyWebpackPlugin({
      patterns: [
        { from: Path.resolve(__dirname, './src/index.html') },
        // { from: Path.resolve(__dirname, './src/img'), to: 'img' },
      ],
    }),
  ],
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
    ],
  },
  resolve: {
    extensions: ['.tsx', '.ts', '.js'],
  },
};
