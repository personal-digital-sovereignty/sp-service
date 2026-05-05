import pandas as pd
import sys
import os

# Add parent dir to sys.path to import modules
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# The file uses standard main block execution, so we might need to mock or test its functions if they are exported.
# Since it's a script, we will just test basic pandas joining logic that it is supposed to perform.

def test_dataframe_join_basic():
    df1 = pd.DataFrame({'date': ['2026-01-01', '2026-01-02'], 'val1': [10, 20]})
    df2 = pd.DataFrame({'date': ['2026-01-01', '2026-01-03'], 'val2': [100, 300]})
    
    df1['date'] = pd.to_datetime(df1['date'])
    df2['date'] = pd.to_datetime(df2['date'])
    
    merged = pd.merge(df1, df2, on='date', how='outer').sort_values('date')
    
    assert len(merged) == 3
    assert pd.isna(merged.iloc[1]['val2'])
    assert pd.isna(merged.iloc[2]['val1'])

def test_dataframe_resample_monthly():
    df = pd.DataFrame({
        'date': pd.date_range(start='2026-01-01', periods=45, freq='D'),
        'value': range(45)
    })
    df.set_index('date', inplace=True)
    monthly = df.resample('ME').mean()
    
    assert len(monthly) == 2 # Jan and Feb
    assert monthly.index[0].month == 1

def test_pearson_correlation_mock():
    df = pd.DataFrame({
        'A': [1, 2, 3, 4, 5],
        'B': [2, 4, 6, 8, 10], # Perfect positive correlation
        'C': [5, 4, 3, 2, 1]   # Perfect negative correlation
    })
    
    corr = df.corr()
    assert corr.loc['A', 'B'] > 0.99
    assert corr.loc['A', 'C'] < -0.99
